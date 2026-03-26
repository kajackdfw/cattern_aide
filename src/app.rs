use std::path::PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use crate::{
    agent::AgentManager,
    config::{self, Config, ProjectConfig},
    state::{
        agent::{AgentKind, AgentState, Provider},
        project::{ConversationMessage, HorizontalTab, Project},
    },
};

// ─── Sidebar selection ────────────────────────────────────────────────────────

pub enum SidebarSelection {
    Project(usize),
    AddButton,
}

// ─── Modal form types ─────────────────────────────────────────────────────────

#[derive(PartialEq)]
pub enum ModalField { Name, Path, Provider, Theme, Confirm }

pub struct NewProjectForm {
    pub name:          String,
    pub path:          String,
    pub provider:      Provider,
    pub theme:         String,
    pub focused_field: ModalField,
    pub name_cursor:   usize,
    pub path_cursor:   usize,
    pub browser:       Option<FolderBrowserState>,
}

impl Default for NewProjectForm {
    fn default() -> Self {
        Self {
            name:          String::new(),
            path:          String::new(),
            provider:      Provider::Anthropic,
            theme:         "dawn".into(),
            focused_field: ModalField::Name,
            name_cursor:   0,
            path_cursor:   0,
            browser:       None,
        }
    }
}

impl NewProjectForm {
    fn next_field(&mut self) {
        self.focused_field = match self.focused_field {
            ModalField::Name     => ModalField::Path,
            ModalField::Path     => ModalField::Provider,
            ModalField::Provider => ModalField::Theme,
            ModalField::Theme    => ModalField::Confirm,
            ModalField::Confirm  => ModalField::Name,
        };
    }

    fn prev_field(&mut self) {
        self.focused_field = match self.focused_field {
            ModalField::Name     => ModalField::Confirm,
            ModalField::Path     => ModalField::Name,
            ModalField::Provider => ModalField::Path,
            ModalField::Theme    => ModalField::Provider,
            ModalField::Confirm  => ModalField::Theme,
        };
    }

    fn toggle_provider(&mut self) {
        self.provider = match self.provider {
            Provider::Anthropic => Provider::OpenCode,
            Provider::OpenCode  => Provider::Anthropic,
        };
    }

    pub fn toggle_theme(&mut self) {
        self.theme = crate::ui::theme::next_theme(&self.theme).to_string();
    }

    fn insert_char(&mut self, ch: char) {
        match self.focused_field {
            ModalField::Name => {
                let byte_pos = char_to_byte(&self.name, self.name_cursor);
                self.name.insert(byte_pos, ch);
                self.name_cursor += 1;
            }
            ModalField::Path => {
                let byte_pos = char_to_byte(&self.path, self.path_cursor);
                self.path.insert(byte_pos, ch);
                self.path_cursor += 1;
            }
            _ => {}
        }
    }

    fn delete_char(&mut self) {
        match self.focused_field {
            ModalField::Name if self.name_cursor > 0 => {
                let end   = char_to_byte(&self.name, self.name_cursor);
                let start = char_to_byte(&self.name, self.name_cursor - 1);
                self.name.drain(start..end);
                self.name_cursor -= 1;
            }
            ModalField::Path if self.path_cursor > 0 => {
                let end   = char_to_byte(&self.path, self.path_cursor);
                let start = char_to_byte(&self.path, self.path_cursor - 1);
                self.path.drain(start..end);
                self.path_cursor -= 1;
            }
            _ => {}
        }
    }
}

// ─── Folder browser ───────────────────────────────────────────────────────────

pub struct FolderBrowserState {
    pub current_dir: PathBuf,
    pub entries:     Vec<PathBuf>,
    pub selected:    usize,
}

impl FolderBrowserState {
    pub fn new(start: PathBuf) -> Self {
        let dir = if start.is_dir() {
            start
        } else {
            start.parent()
                .filter(|p| p.is_dir())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| {
                    std::env::var("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| PathBuf::from("/"))
                })
        };
        let mut s = Self { current_dir: dir, entries: Vec::new(), selected: 0 };
        s.load_entries();
        s
    }

    fn load_entries(&mut self) {
        self.entries.clear();
        self.selected = 0;
        if let Ok(rd) = std::fs::read_dir(&self.current_dir) {
            let mut dirs: Vec<PathBuf> = rd
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .collect();
            dirs.sort();
            self.entries = dirs;
        }
    }

    pub fn enter_selected(&mut self) {
        if let Some(path) = self.entries.get(self.selected).cloned() {
            self.current_dir = path;
            self.load_entries();
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            self.current_dir = parent;
            self.load_entries();
        }
    }

    pub fn move_down(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
        }
    }

    pub fn move_up(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + self.entries.len() - 1) % self.entries.len();
        }
    }
}

fn determine_start_dir(path_str: &str) -> PathBuf {
    let p = PathBuf::from(path_str.trim());
    if p.is_dir() { return p; }
    if let Some(parent) = p.parent() {
        if parent.is_dir() { return parent.to_path_buf(); }
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
}

// ─── Help pages ───────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone)]
pub enum HelpPage { Main, Files, Git, Code, Process }

// ─── AppModal ─────────────────────────────────────────────────────────────────

pub enum AppModal {
    NewProject(NewProjectForm),
    EditProject { idx: usize, form: NewProjectForm },
    DeleteConfirm { idx: usize, name: String, yes_focused: bool },
    GitCommit { project_idx: usize, message: String, cursor: usize },
    Help(HelpPage),
    RecentProjects { selected: usize },
}

// ─── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub projects:          Vec<Project>,
    pub sidebar_sel:       SidebarSelection,
    pub modal:             Option<AppModal>,
    pub config_path:       String,
    pub config:            Config,
    pub agent_manager:     AgentManager,
    pub prompt_focused:         bool,
    pub file_tree_focused:      bool,
    pub git_focused:            bool,
    pub target_code_focused:    bool,
    pub should_quit:            bool,
    /// Set when text is copied; cleared after ~1.5 s in drain_agents tick.
    pub copy_flash:             Option<std::time::Instant>,
}

impl App {
    pub fn from_config(cfg: Config, config_path: &str) -> Self {
        let projects: Vec<Project> = cfg.project.iter().map(|pc| {
            let provider = if pc.provider == "opencode" {
                Provider::OpenCode
            } else {
                Provider::Anthropic
            };
            let mut project = Project::new(pc.name.clone(), provider, pc.path.clone());
            project.theme = pc.theme.clone();
            // Add extra tabs from config
            for tc in &pc.tabs {
                match tc.kind.as_str() {
                    "process" => {
                        let label = if tc.name.is_empty() { "Process".to_string() } else { tc.name.clone() };
                        let kind = if tc.pty {
                            AgentKind::PtyProcess(label.clone())
                        } else {
                            AgentKind::Process(label.clone())
                        };
                        let tab = HorizontalTab::new(kind, &label);
                        let tab = if let Some(cmd) = &tc.command { tab.with_command(cmd) } else { tab };
                        project.tabs.push(tab);
                    }
                    "http_proxy" => {
                        if let (Some(port), Some(target)) = (tc.port, &tc.target) {
                            let label = if tc.name.is_empty() { "HTTP Proxy".to_string() } else { tc.name.clone() };
                            project.tabs.push(HorizontalTab::new(
                                AgentKind::HttpProxy { port, target: target.clone() }, &label,
                            ));
                        }
                    }
                    _ => {}
                }
            }
            project
        }).collect();

        let sidebar_sel = if projects.is_empty() {
            SidebarSelection::AddButton
        } else {
            SidebarSelection::Project(0)
        };

        let mut app = Self {
            projects,
            sidebar_sel,
            modal:             None,
            config_path:       config_path.to_string(),
            config:            cfg,
            agent_manager:     AgentManager::new(),
            prompt_focused:         false,
            file_tree_focused:      false,
            git_focused:            false,
            target_code_focused:    false,
            should_quit:            false,
            copy_flash:             None,
        };
        app.spawn_initial_agents();
        app
    }

    /// Auto-spawn HTTP proxy tabs on startup.
    fn spawn_initial_agents(&mut self) {
        let proxies: Vec<(String, u16, String, AgentKind, String)> = self.projects.iter()
            .flat_map(|p| p.tabs.iter().filter_map(|t| {
                if let AgentKind::HttpProxy { port, target } = &t.kind {
                    Some((p.name.clone(), *port, target.clone(), t.kind.clone(), t.label.clone()))
                } else { None }
            }).collect::<Vec<_>>())
            .collect();
        for (pname, port, target, kind, label) in proxies {
            self.agent_manager.spawn_http_proxy(&pname, port, target, kind, &label);
        }
    }

    pub fn active_project_index(&self) -> Option<usize> {
        match self.sidebar_sel {
            SidebarSelection::Project(i) => Some(i),
            SidebarSelection::AddButton  => None,
        }
    }

    /// Flush pending agent messages into project state. Call each tick.
    pub fn drain_agents(&mut self) {
        let mut projects = std::mem::take(&mut self.projects);
        self.agent_manager.drain_into(&mut projects);
        self.projects = projects;
        // Expire copy flash after 1.5 s
        if let Some(t) = self.copy_flash {
            if t.elapsed() > std::time::Duration::from_millis(1500) {
                self.copy_flash = None;
            }
        }
    }

    pub fn handle_mouse(&mut self, event: MouseEvent, terminal_size: Rect) {
        // Only act on button-down events
        if !matches!(event.kind, MouseEventKind::Down(_)) { return; }
        // Dismiss any open modal on click
        if self.modal.is_some() { return; }

        let panels = crate::ui::layout::panel_rects(terminal_size);
        let (col, row) = (event.column, event.row);
        let hit = |r: Rect| col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height;

        if hit(panels.file_tree) {
            if self.active_project_index().is_some() {
                self.file_tree_focused = true;
                self.git_focused       = false;
                self.prompt_focused    = false;
            }
        } else if hit(panels.git_panel) {
            if self.active_project_index().is_some() {
                self.git_focused       = true;
                self.file_tree_focused = false;
                self.prompt_focused    = false;
            }
        } else if hit(panels.middle) {
            self.file_tree_focused = false;
            self.git_focused       = false;
            if let Some(proj_idx) = self.active_project_index() {
                let is_ai = self.projects.get(proj_idx)
                    .and_then(|p| p.tabs.get(p.active_tab))
                    .map(|t| t.kind == AgentKind::AiPrompt)
                    .unwrap_or(false);
                self.prompt_focused = is_ai;

                if is_ai {
                    self.try_copy_code_block_at(panels.middle, col, row, proj_idx);
                }
            }
        } else if hit(panels.target_code) {
            if self.active_project_index().is_some() {
                self.target_code_focused = true;
                self.file_tree_focused   = false;
                self.git_focused         = false;
                self.prompt_focused      = false;
            }
        } else {
            // sidebar — check for × close buttons first
            let sidebar = panels.sidebar;
            let count   = self.projects.len();
            let mut closed = false;
            for i in 0..count {
                if let Some((bx, by)) = crate::ui::sidebar::close_button_pos(sidebar, count, i) {
                    if col == bx && row == by {
                        self.close_project(i);
                        closed = true;
                        break;
                    }
                }
            }
            if !closed {
                self.file_tree_focused   = false;
                self.git_focused         = false;
                self.prompt_focused      = false;
                self.target_code_focused = false;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.modal.is_some() {
            self.handle_modal_key(key);
        } else {
            self.handle_normal_key(key);
        }
    }

    // ── Normal mode ────────────────────────────────────────────────────────────

    fn handle_normal_key(&mut self, key: KeyEvent) {
        // ── File tree navigation ───────────────────────────────────────────────
        if self.file_tree_focused {
            if let Some(i) = self.active_project_index() {
                match key.code {
                    KeyCode::Esc => { self.file_tree_focused = false; }
                    KeyCode::Char('j') | KeyCode::Down  => { self.projects[i].file_tree.move_down(); }
                    KeyCode::Char('k') | KeyCode::Up    => { self.projects[i].file_tree.move_up(); }
                    KeyCode::Char('g')                  => { self.projects[i].file_tree.move_top(); }
                    KeyCode::Char('G')                  => { self.projects[i].file_tree.move_bottom(); }
                    KeyCode::Enter => {
                        let entry = self.projects[i].file_tree.entries
                            .get(self.projects[i].file_tree.selected)
                            .cloned();
                        if let Some(e) = entry {
                            if e.is_dir {
                                self.projects[i].file_tree.toggle_selected();
                            } else {
                                self.load_file_into_target_code(i, &e.path.clone());
                            }
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Char(' ') => {
                        self.projects[i].file_tree.toggle_selected();
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        // Collapse current entry if expanded, otherwise collapse parent
                        let ft = &mut self.projects[i].file_tree;
                        let sel = ft.selected;
                        if ft.entries.get(sel).map(|e| e.is_dir && e.expanded).unwrap_or(false) {
                            ft.toggle_selected();
                        } else {
                            // Walk up to find parent dir and select it
                            let depth = ft.entries.get(sel).map(|e| e.depth).unwrap_or(0);
                            if depth > 0 {
                                if let Some(parent_idx) = ft.entries[..sel]
                                    .iter().rposition(|e| e.depth < depth && e.is_dir)
                                {
                                    ft.selected = parent_idx;
                                    ft.toggle_selected(); // collapse parent
                                }
                            }
                        }
                    }
                    KeyCode::Char('r') => {
                        let path = self.projects[i].path.clone();
                        self.projects[i].file_tree.reload(&path);
                    }
                    _ => {}
                }
            }
            return;
        }

        // ── Git panel navigation ───────────────────────────────────────────────
        if self.git_focused {
            if let Some(i) = self.active_project_index() {
                match key.code {
                    KeyCode::Esc                                => { self.git_focused = false; }
                    KeyCode::Char('j') | KeyCode::Down          => { self.projects[i].git_status.move_down(); }
                    KeyCode::Char('k') | KeyCode::Up            => { self.projects[i].git_status.move_up(); }
                    KeyCode::Char('r')                          => {
                        let path = self.projects[i].path.clone();
                        self.projects[i].git_status.refresh(&path);
                    }
                    KeyCode::Char('s') => {
                        let (proj_path, file_path) = self.selected_git_file(i);
                        if let Some(fp) = file_path {
                            self.run_git(i, vec!["-C".into(), proj_path, "add".into(), "--".into(), fp]);
                        }
                    }
                    KeyCode::Char('u') => {
                        let (proj_path, file_path) = self.selected_git_file(i);
                        if let Some(fp) = file_path {
                            self.run_git(i, vec!["-C".into(), proj_path, "restore".into(), "--staged".into(), "--".into(), fp]);
                        }
                    }
                    KeyCode::Char('x') => {
                        let (proj_path, file_path) = self.selected_git_file(i);
                        if let Some(fp) = file_path {
                            self.run_git(i, vec!["-C".into(), proj_path, "checkout".into(), "--".into(), fp]);
                        }
                    }
                    KeyCode::Enter => {
                        self.git_diff_selected(i);
                    }
                    KeyCode::Char('p') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "pull".into()]); }
                    KeyCode::Char('P') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "push".into()]); }
                    KeyCode::Char('f') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "fetch".into()]); }
                    KeyCode::Char('c') => {
                        self.git_focused = false;
                        self.modal = Some(AppModal::GitCommit { project_idx: i, message: String::new(), cursor: 0 });
                    }
                    _ => {}
                }
            }
            return;
        }

        // ── Target code scroll/tab navigation (when focused) ──────────────
        if self.target_code_focused {
            if let Some(i) = self.active_project_index() {
                let p = &mut self.projects[i];
                let ntabs = p.target_tabs.len();
                match key.code {
                    KeyCode::Esc => { self.target_code_focused = false; }
                    KeyCode::Char('h') | KeyCode::Left => {
                        if ntabs > 0 {
                            let new = (p.active_target_tab + ntabs - 1) % ntabs;
                            p.active_target_tab = new;
                            // Wrap-around: jump to end of visible range
                            if new == ntabs - 1 { p.target_tab_offset = ntabs.saturating_sub(1); }
                            // Scroll left if active went before the visible window
                            else if new < p.target_tab_offset { p.target_tab_offset = new; }
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        if ntabs > 0 {
                            let new = (p.active_target_tab + 1) % ntabs;
                            p.active_target_tab = new;
                            // Wrap-around: reset to start
                            if new == 0 { p.target_tab_offset = 0; }
                            // Scroll right: the draw function will advance offset as needed
                        }
                    }
                    KeyCode::PageUp => {
                        if let Some(t) = p.target_tabs.get_mut(p.active_target_tab) {
                            t.content.scroll_up(10);
                        }
                    }
                    KeyCode::PageDown => {
                        if let Some(t) = p.target_tabs.get_mut(p.active_target_tab) {
                            t.content.scroll_down(10);
                        }
                    }
                    KeyCode::Char('g') => {
                        if let Some(t) = p.target_tabs.get_mut(p.active_target_tab) {
                            t.content.scroll_offset = 0;
                        }
                    }
                    KeyCode::Char('G') => {
                        if let Some(t) = p.target_tabs.get_mut(p.active_target_tab) {
                            t.content.scroll_offset = usize::MAX;
                        }
                    }
                    KeyCode::Char('x') => {
                        // Close current tab
                        if ntabs > 0 {
                            p.target_tabs.remove(p.active_target_tab);
                            if p.active_target_tab >= p.target_tabs.len() && !p.target_tabs.is_empty() {
                                p.active_target_tab = p.target_tabs.len() - 1;
                            } else if p.target_tabs.is_empty() {
                                p.active_target_tab = 0;
                            }
                        }
                    }
                    _ => {}
                }
            }
            return;
        }

        // ── Prompt input (when on AI Prompt tab and focused) ──────────────────
        if self.prompt_focused {
            if let Some(i) = self.active_project_index() {
                // Check if current tab is PTY — if so, route all keys as raw bytes
                let is_pty_tab = self.projects.get(i)
                    .and_then(|p| p.tabs.get(p.active_tab))
                    .map(|t| matches!(t.kind, AgentKind::PtyProcess(_)))
                    .unwrap_or(false);

                if is_pty_tab {
                    let tab_label = self.projects.get(i)
                        .and_then(|p| p.tabs.get(p.active_tab))
                        .map(|t| t.label.clone());
                    let project_name = self.projects.get(i).map(|p| p.name.clone());
                    if let (Some(label), Some(name)) = (tab_label, project_name) {
                        let bytes: Option<Vec<u8>> = match key.code {
                            // Ctrl+] detaches from PTY
                            KeyCode::Char(']') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.prompt_focused = false;
                                return;
                            }
                            KeyCode::Char(c) => {
                                let mut buf = [0u8; 4];
                                Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
                            }
                            KeyCode::Enter     => Some(b"\r".to_vec()),
                            KeyCode::Backspace => Some(b"\x7f".to_vec()),
                            KeyCode::Tab       => Some(b"\x09".to_vec()),
                            KeyCode::Esc       => Some(b"\x1b".to_vec()),
                            KeyCode::Up        => Some(b"\x1b[A".to_vec()),
                            KeyCode::Down      => Some(b"\x1b[B".to_vec()),
                            KeyCode::Left      => Some(b"\x1b[D".to_vec()),
                            KeyCode::Right     => Some(b"\x1b[C".to_vec()),
                            _ => None,
                        };
                        if let Some(b) = bytes {
                            self.agent_manager.send_pty_stdin(&name, &label, b);
                        }
                    }
                    return;
                }

                match key.code {
                    KeyCode::Esc => {
                        self.prompt_focused = false;
                        return;
                    }
                    KeyCode::Enter => {
                        // AI Prompt tab → submit to AI; Running Process tab → send to stdin
                        let tab_kind = self.projects.get(i)
                            .and_then(|p| p.tabs.get(p.active_tab))
                            .map(|t| t.kind.clone());
                        let tab_label = self.projects.get(i)
                            .and_then(|p| p.tabs.get(p.active_tab))
                            .map(|t| t.label.clone());
                        match tab_kind {
                            Some(AgentKind::AiPrompt) => { self.submit_prompt(i); }
                            Some(AgentKind::Process(_)) => {
                                if let (Some(label), Some(p)) = (tab_label, self.projects.get_mut(i)) {
                                    let text = p.prompt_input.clone() + "\n";
                                    p.prompt_input.clear();
                                    p.prompt_cursor = 0;
                                    self.agent_manager.send_stdin(&p.name.clone(), &label, text);
                                }
                            }
                            _ => {}
                        }
                        return;
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let p = &mut self.projects[i];
                        let byte_pos = char_to_byte(&p.prompt_input, p.prompt_cursor);
                        p.prompt_input.insert(byte_pos, c);
                        p.prompt_cursor += 1;
                        return;
                    }
                    KeyCode::Backspace => {
                        let p = &mut self.projects[i];
                        if p.prompt_cursor > 0 {
                            let end   = char_to_byte(&p.prompt_input, p.prompt_cursor);
                            let start = char_to_byte(&p.prompt_input, p.prompt_cursor - 1);
                            p.prompt_input.drain(start..end);
                            p.prompt_cursor -= 1;
                        }
                        return;
                    }
                    KeyCode::Left => {
                        let p = &mut self.projects[i];
                        p.prompt_cursor = p.prompt_cursor.saturating_sub(1);
                        return;
                    }
                    KeyCode::Right => {
                        let p = &mut self.projects[i];
                        let max = p.prompt_input.chars().count();
                        if p.prompt_cursor < max { p.prompt_cursor += 1; }
                        return;
                    }
                    // Ctrl-K cancels the running agent even when focused
                    KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(p) = self.projects.get(i) {
                            let name  = p.name.clone();
                            let label = p.tabs.get(p.active_tab).map(|t| t.label.clone())
                                .unwrap_or_else(|| "AI Prompt".to_string());
                            self.agent_manager.cancel_tab(&name, &label);
                        }
                        return;
                    }
                    // PgUp/PgDn scroll the output even while input is focused
                    KeyCode::PageUp => {
                        if let Some(p) = self.projects.get_mut(i) {
                            if let Some(tab) = p.tabs.get_mut(p.active_tab) {
                                tab.content.scroll_up(10);
                            }
                        }
                        return;
                    }
                    KeyCode::PageDown => {
                        if let Some(p) = self.projects.get_mut(i) {
                            if let Some(tab) = p.tabs.get_mut(p.active_tab) {
                                tab.content.scroll_down(10);
                            }
                        }
                        return;
                    }
                    _ => { return; }
                }
            }
        }

        // ── Auto-focus running PTY on printable key ────────────────────────────
        if !self.prompt_focused && !self.file_tree_focused && !self.git_focused && !self.target_code_focused {
            if let Some(i) = self.active_project_index() {
                let pty_info = self.projects.get(i).and_then(|p| {
                    p.tabs.get(p.active_tab).and_then(|t| {
                        if matches!(t.kind, AgentKind::PtyProcess(_)) && t.state == AgentState::Running {
                            Some((p.name.clone(), t.label.clone()))
                        } else { None }
                    })
                });
                if let Some((name, label)) = pty_info {
                    let bytes: Option<Vec<u8>> = match key.code {
                        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let mut buf = [0u8; 4];
                            Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
                        }
                        KeyCode::Enter => Some(b"\r".to_vec()),
                        _ => None,
                    };
                    if let Some(b) = bytes {
                        self.prompt_focused = true;
                        self.agent_manager.send_pty_stdin(&name, &label, b);
                        return;
                    }
                }
            }
        }

        // ── Default navigation ─────────────────────────────────────────────────
        match key.code {
            KeyCode::Char('?') => {
                self.modal = Some(AppModal::Help(HelpPage::Main));
            }
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }

            // Ctrl-K: kill running agent on current tab
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(i) = self.active_project_index() {
                    if let Some(p) = self.projects.get(i) {
                        let name  = p.name.clone();
                        let label = p.tabs.get(p.active_tab).map(|t| t.label.clone())
                            .unwrap_or_else(|| "AI Prompt".to_string());
                        self.agent_manager.cancel_tab(&name, &label);
                    }
                }
            }

            // Focus file tree
            KeyCode::Char('f') => {
                if self.active_project_index().is_some() {
                    self.file_tree_focused   = true;
                    self.prompt_focused      = false;
                    self.git_focused         = false;
                    self.target_code_focused = false;
                }
            }

            // Focus git panel
            KeyCode::Char('g') => {
                if self.active_project_index().is_some() {
                    self.git_focused          = true;
                    self.file_tree_focused    = false;
                    self.prompt_focused       = false;
                    self.target_code_focused  = false;
                }
            }

            // Focus target code panel
            KeyCode::Char('t') => {
                if self.active_project_index().is_some() {
                    self.target_code_focused  = true;
                    self.git_focused          = false;
                    self.file_tree_focused    = false;
                    self.prompt_focused       = false;
                }
            }

            // Close selected project (moves to recently-closed, reversible)
            KeyCode::Char('X') | KeyCode::Char('x') => {
                if let SidebarSelection::Project(i) = self.sidebar_sel {
                    self.close_project(i);
                }
            }

            // Open recent projects modal
            KeyCode::Char('R') => {
                if !self.config.recently_closed.is_empty() {
                    self.modal = Some(AppModal::RecentProjects { selected: 0 });
                }
            }

            // Project navigation
            KeyCode::Char('j') | KeyCode::Down => {
                let total = self.projects.len() + 1;
                let cur = match self.sidebar_sel {
                    SidebarSelection::Project(i) => i,
                    SidebarSelection::AddButton  => self.projects.len(),
                };
                let next = (cur + 1) % total;
                self.sidebar_sel = if next == self.projects.len() {
                    SidebarSelection::AddButton
                } else {
                    SidebarSelection::Project(next)
                };
                self.prompt_focused      = false;
                self.file_tree_focused   = false;
                self.git_focused         = false;
                self.target_code_focused = false;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let total = self.projects.len() + 1;
                let cur = match self.sidebar_sel {
                    SidebarSelection::Project(i) => i,
                    SidebarSelection::AddButton  => self.projects.len(),
                };
                let prev = (cur + total - 1) % total;
                self.sidebar_sel = if prev == self.projects.len() {
                    SidebarSelection::AddButton
                } else {
                    SidebarSelection::Project(prev)
                };
                self.prompt_focused      = false;
                self.file_tree_focused   = false;
                self.git_focused         = false;
                self.target_code_focused = false;
            }

            // Open modal when "+" is selected; focus prompt input on AI tab; start process tabs
            KeyCode::Enter | KeyCode::Char(' ') => {
                match self.sidebar_sel {
                    SidebarSelection::AddButton => {
                        self.modal = Some(AppModal::NewProject(NewProjectForm::default()));
                    }
                    SidebarSelection::Project(i) => {
                        let tab_kind = self.projects.get(i)
                            .and_then(|p| p.tabs.get(p.active_tab))
                            .map(|t| t.kind.clone());
                        match tab_kind {
                            Some(AgentKind::AiPrompt) => { self.prompt_focused = true; }
                            Some(AgentKind::Process(_)) => {
                                let is_running = self.projects.get(i)
                                    .and_then(|p| p.tabs.get(p.active_tab))
                                    .map(|t| t.state == AgentState::Running)
                                    .unwrap_or(false);
                                if is_running {
                                    self.prompt_focused = true;
                                } else {
                                    self.start_process_tab(i);
                                }
                            }
                            Some(AgentKind::PtyProcess(_)) => {
                                let is_running = self.projects.get(i)
                                    .and_then(|p| p.tabs.get(p.active_tab))
                                    .map(|t| t.state == AgentState::Running)
                                    .unwrap_or(false);
                                if is_running {
                                    self.prompt_focused = true;
                                } else {
                                    self.start_process_tab(i);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Edit project
            KeyCode::Char('e') => {
                if let SidebarSelection::Project(i) = self.sidebar_sel {
                    if let Some(p) = self.projects.get(i) {
                        let nc = p.name.chars().count();
                        let pc = p.path.chars().count();
                        let form = NewProjectForm {
                            name:          p.name.clone(),
                            path:          p.path.clone(),
                            provider:      p.provider.clone(),
                            theme:         p.theme.clone(),
                            focused_field: ModalField::Name,
                            name_cursor:   nc,
                            path_cursor:   pc,
                            browser:       None,
                        };
                        self.modal = Some(AppModal::EditProject { idx: i, form });
                    }
                }
            }

            // Delete project
            KeyCode::Char('d') | KeyCode::Delete => {
                if let SidebarSelection::Project(i) = self.sidebar_sel {
                    if let Some(p) = self.projects.get(i) {
                        let name = p.name.clone();
                        self.modal = Some(AppModal::DeleteConfirm { idx: i, name, yes_focused: false });
                    }
                }
            }

            // Tab navigation
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                self.prompt_focused    = false;
                self.file_tree_focused = false;
                self.git_focused       = false;
                if let Some(i) = self.active_project_index() {
                    if let Some(p) = self.projects.get_mut(i) {
                        if !p.tabs.is_empty() {
                            p.active_tab = (p.active_tab + 1) % p.tabs.len();
                        }
                    }
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.prompt_focused = false;
                if let Some(i) = self.active_project_index() {
                    if let Some(p) = self.projects.get_mut(i) {
                        if !p.tabs.is_empty() {
                            let n = p.tabs.len();
                            p.active_tab = (p.active_tab + n - 1) % n;
                        }
                    }
                }
            }
            KeyCode::PageUp => {
                if let Some(i) = self.active_project_index() {
                    if let Some(p) = self.projects.get_mut(i) {
                        if let Some(tab) = p.tabs.get_mut(p.active_tab) {
                            tab.content.scroll_up(10);
                        }
                    }
                }
            }
            KeyCode::PageDown => {
                if let Some(i) = self.active_project_index() {
                    if let Some(p) = self.projects.get_mut(i) {
                        if let Some(tab) = p.tabs.get_mut(p.active_tab) {
                            tab.content.scroll_down(10);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn submit_prompt(&mut self, project_idx: usize) {
        let (pname, provider, path, prompt) = {
            let p = &self.projects[project_idx];
            let pr = p.prompt_input.trim().to_string();
            if pr.is_empty() { return; }
            (p.name.clone(), p.provider.clone(), p.path.clone(), pr)
        };

        // Append user turn to conversation history
        self.projects[project_idx].conversation.push(ConversationMessage {
            role:    "user".to_string(),
            content: prompt.clone(),
        });

        // Visual separator in the output pane
        if let Some(tab) = self.projects[project_idx].tabs.iter_mut()
            .find(|t| t.kind == AgentKind::AiPrompt)
        {
            tab.content.push_line(format!("▶ {prompt}"));
            tab.state = AgentState::Running;
        }

        self.projects[project_idx].prompt_input.clear();
        self.projects[project_idx].prompt_cursor = 0;
        self.prompt_focused = false;

        let conversation = self.projects[project_idx].conversation.clone();
        let cfg = &self.config;
        self.agent_manager.spawn_ai_prompt(&pname, &provider, &path, conversation, cfg);
    }

    // ── Modal mode ─────────────────────────────────────────────────────────────

    fn handle_modal_key(&mut self, key: KeyEvent) {
        if matches!(self.modal, Some(AppModal::Help(_))) {
            let is_main = matches!(self.modal, Some(AppModal::Help(HelpPage::Main)));
            self.modal = if is_main {
                match key.code {
                    KeyCode::Char('f') => Some(AppModal::Help(HelpPage::Files)),
                    KeyCode::Char('g') => Some(AppModal::Help(HelpPage::Git)),
                    KeyCode::Char('c') => Some(AppModal::Help(HelpPage::Code)),
                    KeyCode::Char('p') => Some(AppModal::Help(HelpPage::Process)),
                    _                  => None,
                }
            } else {
                match key.code {
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('b') => {
                        Some(AppModal::Help(HelpPage::Main))
                    }
                    _ => None,
                }
            };
            return;
        }

        if matches!(self.modal, Some(AppModal::RecentProjects { .. })) {
            self.handle_recent_projects_key(key);
            return;
        }

        if matches!(self.modal, Some(AppModal::DeleteConfirm { .. })) {
            self.handle_delete_key(key);
            return;
        }

        if matches!(self.modal, Some(AppModal::GitCommit { .. })) {
            self.handle_git_commit_key(key);
            return;
        }

        let mut do_confirm = false;

        {
            // Both NewProject and EditProject use NewProjectForm — extract shared ref
            let form = match &mut self.modal {
                Some(AppModal::NewProject(f))            => f,
                Some(AppModal::EditProject { form: f, .. }) => f,
                _ => return,
            };

            // ── Folder browser is open ─────────────────────────────────────────
            if form.browser.is_some() {
                enum BrowserOp { None, Close, Down, Up, Enter, GoUp, Select }
                let op = match key.code {
                    KeyCode::Esc                                              => BrowserOp::Close,
                    KeyCode::Char('j') | KeyCode::Down                        => BrowserOp::Down,
                    KeyCode::Char('k') | KeyCode::Up                          => BrowserOp::Up,
                    KeyCode::Enter                                            => BrowserOp::Enter,
                    KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h')  => BrowserOp::GoUp,
                    KeyCode::Char(' ')                                        => BrowserOp::Select,
                    _ => BrowserOp::None,
                };
                match op {
                    BrowserOp::Close  => { form.browser = None; }
                    BrowserOp::Down   => { form.browser.as_mut().unwrap().move_down(); }
                    BrowserOp::Up     => { form.browser.as_mut().unwrap().move_up(); }
                    BrowserOp::GoUp   => { form.browser.as_mut().unwrap().go_up(); }
                    BrowserOp::Enter  => {
                        let b = form.browser.as_mut().unwrap();
                        if b.entries.is_empty() {
                            let sel = b.current_dir.to_string_lossy().to_string();
                            form.path = sel;
                            form.path_cursor = form.path.chars().count();
                            form.browser = None;
                        } else {
                            b.enter_selected();
                        }
                    }
                    BrowserOp::Select => {
                        let sel = form.browser.as_ref().unwrap().current_dir.to_string_lossy().to_string();
                        form.path = sel;
                        form.path_cursor = form.path.chars().count();
                        form.browser = None;
                    }
                    BrowserOp::None => {}
                }
                return;
            }

            // ── Normal form key handling ───────────────────────────────────────
            match key.code {
                KeyCode::Esc => {
                    self.modal = None;
                    return;
                }
                KeyCode::Tab | KeyCode::Down => {
                    form.next_field();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    form.prev_field();
                }
                KeyCode::Left | KeyCode::Right
                    if form.focused_field == ModalField::Provider =>
                {
                    form.toggle_provider();
                }
                KeyCode::Left | KeyCode::Right
                    if form.focused_field == ModalField::Theme =>
                {
                    form.toggle_theme();
                }
                KeyCode::Enter if form.focused_field == ModalField::Confirm => {
                    do_confirm = true;
                }
                KeyCode::Enter => {
                    form.next_field();
                }
                KeyCode::Char('f')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && form.focused_field == ModalField::Path =>
                {
                    let start = determine_start_dir(&form.path);
                    form.browser = Some(FolderBrowserState::new(start));
                }
                KeyCode::Char(c) => {
                    form.insert_char(c);
                }
                KeyCode::Backspace => {
                    form.delete_char();
                }
                _ => {}
            }
        }

        if do_confirm {
            match &self.modal {
                Some(AppModal::NewProject(_))                 => self.confirm_new_project(),
                Some(AppModal::EditProject { idx, .. })       => { let i = *idx; self.confirm_edit_project(i); }
                _ => {}
            }
        }
    }

    fn handle_git_commit_key(&mut self, key: KeyEvent) {
        let Some(AppModal::GitCommit { message, cursor, .. }) = &mut self.modal else { return };
        match key.code {
            KeyCode::Esc       => { self.modal = None; }
            KeyCode::Enter     => { self.confirm_git_commit(); }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let bp = char_to_byte(message, *cursor);
                message.insert(bp, c);
                *cursor += 1;
            }
            KeyCode::Backspace => {
                if *cursor > 0 {
                    let end   = char_to_byte(message, *cursor);
                    let start = char_to_byte(message, *cursor - 1);
                    message.drain(start..end);
                    *cursor -= 1;
                }
            }
            KeyCode::Left  => { *cursor = cursor.saturating_sub(1); }
            KeyCode::Right => {
                let max = message.chars().count();
                if *cursor < max { *cursor += 1; }
            }
            _ => {}
        }
    }

    fn handle_delete_key(&mut self, key: KeyEvent) {
        enum Op { None, Cancel, Toggle, Confirm }
        let op = {
            let Some(AppModal::DeleteConfirm { yes_focused, .. }) = &self.modal else { return };
            match key.code {
                KeyCode::Esc
                | KeyCode::Char('n') | KeyCode::Char('N') => Op::Cancel,
                KeyCode::Left | KeyCode::Right | KeyCode::Tab => Op::Toggle,
                KeyCode::Enter => if *yes_focused { Op::Confirm } else { Op::Cancel },
                KeyCode::Char('y') | KeyCode::Char('Y') => Op::Confirm,
                _ => Op::None,
            }
        };
        match op {
            Op::Cancel  => { self.modal = None; }
            Op::Toggle  => {
                if let Some(AppModal::DeleteConfirm { yes_focused, .. }) = &mut self.modal {
                    *yes_focused = !*yes_focused;
                }
            }
            Op::Confirm => { self.confirm_delete_project(); }
            Op::None    => {}
        }
    }

    fn confirm_new_project(&mut self) {
        let (name, path, provider_str, theme_str) = {
            let Some(AppModal::NewProject(ref form)) = self.modal else { return };
            if form.name.trim().is_empty() { return; }
            let p = match form.provider {
                Provider::Anthropic => "anthropic",
                Provider::OpenCode  => "opencode",
            };
            (form.name.trim().to_string(), form.path.trim().to_string(), p.to_string(), form.theme.clone())
        };

        let provider = if provider_str == "opencode" {
            Provider::OpenCode
        } else {
            Provider::Anthropic
        };

        let mut project = Project::new(name.clone(), provider, path.clone());
        project.theme = theme_str.clone();
        self.projects.push(project);
        self.sidebar_sel = SidebarSelection::Project(self.projects.len() - 1);
        self.modal = None;

        let cfg_path = self.config_path.clone();
        match config::load(&cfg_path) {
            Ok(mut cfg) => {
                cfg.project.push(ProjectConfig { name, provider: provider_str, path, theme: theme_str, tabs: vec![] });
                if let Err(e) = config::save(&cfg, &cfg_path) {
                    tracing::error!("save config: {e}");
                }
            }
            Err(_) => {
                let cfg = Config {
                    general:          config::default_config().general,
                    provider:         config::default_config().provider,
                    recently_closed:  self.config.recently_closed.clone(),
                    project:          self.projects.iter().map(|p| ProjectConfig {
                        name:     p.name.clone(),
                        provider: match p.provider {
                            Provider::Anthropic => "anthropic".into(),
                            Provider::OpenCode  => "opencode".into(),
                        },
                        path:  p.path.clone(),
                        theme: p.theme.clone(),
                        tabs:  vec![],
                    }).collect(),
                };
                if let Err(e) = config::save(&cfg, &cfg_path) {
                    tracing::error!("create config: {e}");
                }
            }
        }
    }

    /// Returns `(project_path, Option<cleaned_file_path>)` for the selected git file.
    fn selected_git_file(&self, project_idx: usize) -> (String, Option<String>) {
        let p = match self.projects.get(project_idx) { Some(p) => p, None => return (String::new(), None) };
        let proj_path = p.path.clone();
        let file_path = p.git_status.files.get(p.git_status.selected)
            .map(|f| git_working_path(&f.path).to_string());
        (proj_path, file_path)
    }

    /// Run `git diff` (or `git diff --staged`) on the selected file and open it in the right panel.
    fn git_diff_selected(&mut self, project_idx: usize) {
        use crate::state::git::FileStatusCode;
        let (proj_path, file_path, staged, label) = {
            let p = match self.projects.get(project_idx) { Some(p) => p, None => return };
            let file = match p.git_status.files.get(p.git_status.selected) { Some(f) => f, None => return };
            let fp = git_working_path(&file.path).to_string();
            let staged = matches!(&file.status,
                FileStatusCode::Staged | FileStatusCode::Added | FileStatusCode::Renamed);
            if matches!(&file.status, FileStatusCode::Untracked) { return; }
            let short = std::path::Path::new(&fp)
                .file_name().and_then(|n| n.to_str()).unwrap_or(&fp).to_string();
            (p.path.clone(), fp, staged, format!("M {short}"))
        };

        let mut cmd = std::process::Command::new("git");
        cmd.arg("-C").arg(&proj_path).arg("diff");
        if staged { cmd.arg("--staged"); }
        cmd.arg("--").arg(&file_path);

        let output = match cmd.output() {
            Ok(o)  => o,
            Err(_) => return,
        };
        let text = String::from_utf8_lossy(&output.stdout);
        if text.trim().is_empty() { return; }

        let p = match self.projects.get_mut(project_idx) { Some(p) => p, None => return };
        push_target_tab(p, label, &text, true);
        self.target_code_focused = true;
        self.git_focused         = false;
    }

    /// Load a file from the filesystem into the right-column viewer as a new tab.
    fn load_file_into_target_code(&mut self, project_idx: usize, path: &std::path::Path) {
        let Ok(content) = std::fs::read_to_string(path) else { return };
        let label = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let p = match self.projects.get_mut(project_idx) { Some(p) => p, None => return };
        push_target_tab(p, label, &content, false);
        self.file_tree_focused = false;
    }

    /// Start the Process tab on the active tab if idle/error.
    fn start_process_tab(&mut self, project_idx: usize) {
        let p = match self.projects.get(project_idx) { Some(p) => p, None => return };
        let tab = match p.tabs.get(p.active_tab) { Some(t) => t, None => return };
        if !matches!(tab.state, AgentState::Idle | AgentState::Error) { return; }
        let pname  = p.name.clone();
        let ppath  = p.path.clone();
        let kind   = tab.kind.clone();
        let label  = tab.label.clone();
        let cmd    = tab.command.clone();
        match &kind {
            AgentKind::Process(_) => {
                if let Some(cmd) = cmd {
                    self.agent_manager.spawn_process(&pname, &ppath, &cmd, kind, &label);
                }
            }
            AgentKind::PtyProcess(_) => {
                if let Some(cmd) = cmd {
                    self.agent_manager.spawn_pty_process(&pname, &ppath, &cmd, AgentKind::PtyProcess(label.clone()), &label, 200, 50);
                }
            }
            _ => {}
        }
    }

    fn run_git(&mut self, project_idx: usize, git_args: Vec<String>) {
        if let Some(p) = self.projects.get(project_idx) {
            let name = p.name.clone();
            let path = p.path.clone();
            // Switch to Git tab so output is visible
            if let Some(p) = self.projects.get_mut(project_idx) {
                if let Some(idx) = p.tabs.iter().position(|t| t.kind == AgentKind::Git) {
                    p.active_tab = idx;
                }
            }
            self.agent_manager.spawn_git_command(&name, &path, git_args);
        }
    }

    fn confirm_git_commit(&mut self) {
        let (project_idx, message) = match &self.modal {
            Some(AppModal::GitCommit { project_idx, message, .. }) => {
                let msg = message.trim().to_string();
                if msg.is_empty() { return; }
                (*project_idx, msg)
            }
            _ => return,
        };
        self.modal = None;
        let path = self.projects.get(project_idx).map(|p| p.path.clone()).unwrap_or_default();
        self.run_git(project_idx, vec![
            "-C".into(), path,
            "commit".into(), "-m".into(), message,
        ]);
    }

    fn confirm_edit_project(&mut self, idx: usize) {
        let (name, path, provider_str, theme_str) = {
            let Some(AppModal::EditProject { form, .. }) = &self.modal else { return };
            if form.name.trim().is_empty() { return; }
            let p = match form.provider {
                Provider::Anthropic => "anthropic",
                Provider::OpenCode  => "opencode",
            };
            (form.name.trim().to_string(), form.path.trim().to_string(), p.to_string(), form.theme.clone())
        };

        let provider = if provider_str == "opencode" { Provider::OpenCode } else { Provider::Anthropic };

        if let Some(project) = self.projects.get_mut(idx) {
            project.name     = name.clone();
            project.path     = path.clone();
            project.provider = provider;
            project.theme    = theme_str.clone();
        }
        self.modal = None;

        let cfg_path = self.config_path.clone();
        match config::load(&cfg_path) {
            Ok(mut cfg) => {
                if let Some(pc) = cfg.project.get_mut(idx) {
                    pc.name     = name;
                    pc.provider = provider_str;
                    pc.path     = path;
                    pc.theme    = theme_str;
                }
                if let Err(e) = config::save(&cfg, &cfg_path) {
                    tracing::error!("save config (edit): {e}");
                }
            }
            Err(e) => tracing::error!("load config (edit): {e}"),
        }
    }

    fn confirm_delete_project(&mut self) {
        let idx = match &self.modal {
            Some(AppModal::DeleteConfirm { idx, .. }) => *idx,
            _ => return,
        };

        if idx < self.projects.len() {
            self.projects.remove(idx);
        }

        self.sidebar_sel = if self.projects.is_empty() {
            SidebarSelection::AddButton
        } else {
            SidebarSelection::Project(idx.saturating_sub(1).min(self.projects.len() - 1))
        };
        self.modal             = None;
        self.prompt_focused    = false;
        self.file_tree_focused = false;
        self.git_focused       = false;

        let cfg_path = self.config_path.clone();
        match config::load(&cfg_path) {
            Ok(mut cfg) => {
                if idx < cfg.project.len() {
                    cfg.project.remove(idx);
                }
                if let Err(e) = config::save(&cfg, &cfg_path) {
                    tracing::error!("save config (delete): {e}");
                }
            }
            Err(e) => tracing::error!("load config (delete): {e}"),
        }
    }

    // ── Close / Recent projects ────────────────────────────────────────────────

    /// Close a project: remove from active list, push to recently-closed.
    /// If the click at (col, row) falls inside a fenced code block in the AI output,
    /// copy its content to the clipboard via OSC 52 and set the copy flash.
    fn try_copy_code_block_at(&mut self, middle: Rect, col: u16, row: u16, proj_idx: usize) {
        use ratatui::layout::{Constraint, Direction, Layout};
        // Replicate the layout from main_area::draw_main
        let mid = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(middle);
        let content_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)])
            .split(mid[1]);
        let output_area = content_split[1];

        // Click must be inside the output content (inside the 1-cell border)
        if row <= output_area.y || row >= output_area.y + output_area.height.saturating_sub(1) { return; }
        if col <= output_area.x || col >= output_area.x + output_area.width.saturating_sub(1) { return; }

        let visual_line = (row - output_area.y - 1) as usize;

        let Some(project) = self.projects.get(proj_idx) else { return };
        let Some(tab)     = project.tabs.get(project.active_tab) else { return };
        let visible  = output_area.height.saturating_sub(2) as usize;
        let offset   = tab.content.clamped_offset(visible);
        let logical  = offset + visual_line;

        let blocks = crate::clipboard::extract_code_blocks(&tab.content.lines);
        if let Some(block) = crate::clipboard::block_at_line(&blocks, logical) {
            crate::clipboard::copy_to_clipboard(&block.content);
            self.copy_flash = Some(std::time::Instant::now());
        }
    }

    fn close_project(&mut self, idx: usize) {
        if idx >= self.projects.len() { return; }
        let p = &self.projects[idx];
        let provider_str = match p.provider {
            Provider::Anthropic => "anthropic",
            Provider::OpenCode  => "opencode",
        };
        let pc = config::ProjectConfig {
            name:     p.name.clone(),
            provider: provider_str.into(),
            path:     p.path.clone(),
            theme:    p.theme.clone(),
            tabs:     vec![],
        };

        self.projects.remove(idx);
        self.sidebar_sel = if self.projects.is_empty() {
            SidebarSelection::AddButton
        } else {
            SidebarSelection::Project(idx.saturating_sub(1).min(self.projects.len() - 1))
        };
        self.prompt_focused    = false;
        self.file_tree_focused = false;
        self.git_focused       = false;

        let cfg_path = self.config_path.clone();
        if let Ok(mut cfg) = config::load(&cfg_path) {
            if idx < cfg.project.len() { cfg.project.remove(idx); }
            // Deduplicate: remove any existing entry with same name before prepending
            cfg.recently_closed.retain(|r| r.name != pc.name);
            cfg.recently_closed.insert(0, pc.clone());
            if cfg.recently_closed.len() > config::RECENT_MAX {
                cfg.recently_closed.truncate(config::RECENT_MAX);
            }
            // Keep in-memory config in sync
            self.config.recently_closed = cfg.recently_closed.clone();
            self.config.project = cfg.project.clone();
            let _ = config::save(&cfg, &cfg_path);
        }
    }

    fn handle_recent_projects_key(&mut self, key: KeyEvent) {
        let n = self.config.recently_closed.len();
        if n == 0 { self.modal = None; return; }

        let selected = match &self.modal {
            Some(AppModal::RecentProjects { selected }) => *selected,
            _ => return,
        };

        match key.code {
            KeyCode::Esc => { self.modal = None; }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(AppModal::RecentProjects { selected }) = &mut self.modal {
                    *selected = (*selected + 1).min(n - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(AppModal::RecentProjects { selected }) = &mut self.modal {
                    *selected = selected.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                self.reopen_recent(selected);
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                self.remove_from_recent(selected);
            }
            _ => {}
        }
    }

    /// Reopen a recently-closed project at position `idx` in the recent list.
    fn reopen_recent(&mut self, idx: usize) {
        let pc = match self.config.recently_closed.get(idx).cloned() {
            Some(pc) => pc,
            None => return,
        };
        let provider = if pc.provider == "opencode" { Provider::OpenCode } else { Provider::Anthropic };
        let mut project = Project::new(pc.name.clone(), provider, pc.path.clone());
        project.theme = pc.theme.clone();
        self.projects.push(project);
        self.sidebar_sel = SidebarSelection::Project(self.projects.len() - 1);
        self.modal = None;

        let cfg_path = self.config_path.clone();
        if let Ok(mut cfg) = config::load(&cfg_path) {
            cfg.recently_closed.remove(idx);
            cfg.project.push(pc);
            self.config.recently_closed = cfg.recently_closed.clone();
            self.config.project = cfg.project.clone();
            let _ = config::save(&cfg, &cfg_path);
        }
    }

    /// Permanently remove an entry from the recent list (does not restore it).
    fn remove_from_recent(&mut self, idx: usize) {
        let cfg_path = self.config_path.clone();
        if let Ok(mut cfg) = config::load(&cfg_path) {
            if idx < cfg.recently_closed.len() {
                cfg.recently_closed.remove(idx);
            }
            self.config.recently_closed = cfg.recently_closed.clone();
            let _ = config::save(&cfg, &cfg_path);
        }
        // Adjust selection if needed
        let n = self.config.recently_closed.len();
        if n == 0 {
            self.modal = None;
        } else if let Some(AppModal::RecentProjects { selected }) = &mut self.modal {
            if *selected >= n { *selected = n - 1; }
        }
    }
}

/// Convert a char-count cursor position to a byte offset.
fn char_to_byte(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// Prepend a new tab (or refresh + move-to-front if one with the same label exists).
fn push_target_tab(p: &mut crate::state::project::Project, label: String, text: &str, is_diff: bool) {
    use crate::state::{container::TextContainer, project::TargetTab};

    // If a tab with this label already exists, remove it so we can re-insert at front.
    if let Some(pos) = p.target_tabs.iter().position(|t| t.label == label) {
        p.target_tabs.remove(pos);
    }

    let mut content = TextContainer::new();
    for line in text.lines() {
        content.push_line(line.to_string());
    }
    p.target_tabs.insert(0, TargetTab { label, content, is_diff });
    p.active_target_tab = 0;
    p.target_tab_offset = 0;
}

/// For renamed files, git porcelain v1 shows "old -> new"; return the destination.
fn git_working_path(path: &str) -> &str {
    if let Some(idx) = path.rfind(" -> ") {
        return &path[idx + 4..];
    }
    path
}
