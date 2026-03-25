use std::path::PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::{
    agent::AgentManager,
    config::{self, Config, ProjectConfig},
    state::{agent::{AgentKind, AgentState, Provider}, project::{ConversationMessage, Project}},
};

// ─── Sidebar selection ────────────────────────────────────────────────────────

pub enum SidebarSelection {
    Project(usize),
    AddButton,
}

// ─── Modal form types ─────────────────────────────────────────────────────────

#[derive(PartialEq)]
pub enum ModalField { Name, Path, Provider, Confirm }

pub struct NewProjectForm {
    pub name:          String,
    pub path:          String,
    pub provider:      Provider,
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
            ModalField::Provider => ModalField::Confirm,
            ModalField::Confirm  => ModalField::Name,
        };
    }

    fn prev_field(&mut self) {
        self.focused_field = match self.focused_field {
            ModalField::Name     => ModalField::Confirm,
            ModalField::Path     => ModalField::Name,
            ModalField::Provider => ModalField::Path,
            ModalField::Confirm  => ModalField::Provider,
        };
    }

    fn toggle_provider(&mut self) {
        self.provider = match self.provider {
            Provider::Anthropic => Provider::OpenCode,
            Provider::OpenCode  => Provider::Anthropic,
        };
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

// ─── AppModal ─────────────────────────────────────────────────────────────────

pub enum AppModal {
    NewProject(NewProjectForm),
    EditProject { idx: usize, form: NewProjectForm },
    DeleteConfirm { idx: usize, name: String, yes_focused: bool },
    GitCommit { project_idx: usize, message: String, cursor: usize },
    Help,
}

// ─── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub projects:          Vec<Project>,
    pub sidebar_sel:       SidebarSelection,
    pub modal:             Option<AppModal>,
    pub config_path:       String,
    pub config:            Config,
    pub agent_manager:     AgentManager,
    pub prompt_focused:    bool,
    pub file_tree_focused: bool,
    pub git_focused:       bool,
    pub should_quit:       bool,
}

impl App {
    pub fn from_config(cfg: Config, config_path: &str) -> Self {
        let projects: Vec<Project> = cfg.project.iter().map(|pc| {
            let provider = if pc.provider == "opencode" {
                Provider::OpenCode
            } else {
                Provider::Anthropic
            };
            Project::new(pc.name.clone(), provider, pc.path.clone())
        }).collect();

        let sidebar_sel = if projects.is_empty() {
            SidebarSelection::AddButton
        } else {
            SidebarSelection::Project(0)
        };

        Self {
            projects,
            sidebar_sel,
            modal:             None,
            config_path:       config_path.to_string(),
            config:            cfg,
            agent_manager:     AgentManager::new(),
            prompt_focused:    false,
            file_tree_focused: false,
            git_focused:       false,
            should_quit:       false,
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
                    KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right | KeyCode::Char(' ') => {
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
                    KeyCode::Char('s') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "add".into(), "-A".into()]); }
                    KeyCode::Char('u') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "reset".into(), "HEAD".into()]); }
                    KeyCode::Char('p') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "pull".into()]); }
                    KeyCode::Char('P') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "push".into()]); }
                    KeyCode::Char('f') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "fetch".into()]); }
                    KeyCode::Char('x') => { self.run_git(i, vec!["-C".into(), self.projects[i].path.clone(), "checkout".into(), "--".into(), ".".into()]); }
                    KeyCode::Char('c') => {
                        self.git_focused = false;
                        self.modal = Some(AppModal::GitCommit { project_idx: i, message: String::new(), cursor: 0 });
                    }
                    _ => {}
                }
            }
            return;
        }

        // ── Prompt input (when on AI Prompt tab and focused) ──────────────────
        if self.prompt_focused {
            if let Some(i) = self.active_project_index() {
                match key.code {
                    KeyCode::Esc => {
                        self.prompt_focused = false;
                        return;
                    }
                    KeyCode::Enter => {
                        self.submit_prompt(i);
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
                            let name = p.name.clone();
                            self.agent_manager.cancel_ai_prompt(&name);
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

        // ── Default navigation ─────────────────────────────────────────────────
        match key.code {
            KeyCode::Char('?') => {
                self.modal = Some(AppModal::Help);
            }
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }

            // Ctrl-K: kill running agent on current tab
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(i) = self.active_project_index() {
                    if let Some(p) = self.projects.get(i) {
                        let name = p.name.clone();
                        self.agent_manager.cancel_ai_prompt(&name);
                    }
                }
            }

            // Focus file tree
            KeyCode::Char('f') => {
                if self.active_project_index().is_some() {
                    self.file_tree_focused = true;
                    self.prompt_focused    = false;
                    self.git_focused       = false;
                }
            }

            // Focus git panel
            KeyCode::Char('g') => {
                if self.active_project_index().is_some() {
                    self.git_focused       = true;
                    self.file_tree_focused = false;
                    self.prompt_focused    = false;
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
                self.prompt_focused    = false;
                self.file_tree_focused = false;
                self.git_focused       = false;
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
                self.prompt_focused    = false;
                self.file_tree_focused = false;
                self.git_focused       = false;
            }

            // Open modal when "+" is selected; focus prompt input on AI tab
            KeyCode::Enter | KeyCode::Char(' ') => {
                match self.sidebar_sel {
                    SidebarSelection::AddButton => {
                        self.modal = Some(AppModal::NewProject(NewProjectForm::default()));
                    }
                    SidebarSelection::Project(i) => {
                        if let Some(p) = self.projects.get(i) {
                            let is_ai = p.tabs.get(p.active_tab)
                                .map(|t| t.kind == AgentKind::AiPrompt)
                                .unwrap_or(false);
                            if is_ai { self.prompt_focused = true; }
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
        if matches!(self.modal, Some(AppModal::Help)) {
            self.modal = None;
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
        let (name, path, provider_str) = {
            let Some(AppModal::NewProject(ref form)) = self.modal else { return };
            if form.name.trim().is_empty() { return; }
            let p = match form.provider {
                Provider::Anthropic => "anthropic",
                Provider::OpenCode  => "opencode",
            };
            (form.name.trim().to_string(), form.path.trim().to_string(), p.to_string())
        };

        let provider = if provider_str == "opencode" {
            Provider::OpenCode
        } else {
            Provider::Anthropic
        };

        self.projects.push(Project::new(name.clone(), provider, path.clone()));
        self.sidebar_sel = SidebarSelection::Project(self.projects.len() - 1);
        self.modal = None;

        let cfg_path = self.config_path.clone();
        match config::load(&cfg_path) {
            Ok(mut cfg) => {
                cfg.project.push(ProjectConfig { name, provider: provider_str, path });
                if let Err(e) = config::save(&cfg, &cfg_path) {
                    tracing::error!("save config: {e}");
                }
            }
            Err(_) => {
                let cfg = Config {
                    general:  config::default_config().general,
                    provider: config::default_config().provider,
                    project:  self.projects.iter().map(|p| ProjectConfig {
                        name:     p.name.clone(),
                        provider: match p.provider {
                            Provider::Anthropic => "anthropic".into(),
                            Provider::OpenCode  => "opencode".into(),
                        },
                        path: p.path.clone(),
                    }).collect(),
                };
                if let Err(e) = config::save(&cfg, &cfg_path) {
                    tracing::error!("create config: {e}");
                }
            }
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
        let (name, path, provider_str) = {
            let Some(AppModal::EditProject { form, .. }) = &self.modal else { return };
            if form.name.trim().is_empty() { return; }
            let p = match form.provider {
                Provider::Anthropic => "anthropic",
                Provider::OpenCode  => "opencode",
            };
            (form.name.trim().to_string(), form.path.trim().to_string(), p.to_string())
        };

        let provider = if provider_str == "opencode" { Provider::OpenCode } else { Provider::Anthropic };

        if let Some(project) = self.projects.get_mut(idx) {
            project.name     = name.clone();
            project.path     = path.clone();
            project.provider = provider;
        }
        self.modal = None;

        let cfg_path = self.config_path.clone();
        match config::load(&cfg_path) {
            Ok(mut cfg) => {
                if let Some(pc) = cfg.project.get_mut(idx) {
                    pc.name     = name;
                    pc.provider = provider_str;
                    pc.path     = path;
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
}

/// Convert a char-count cursor position to a byte offset.
fn char_to_byte(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}
