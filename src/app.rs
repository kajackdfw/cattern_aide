use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::{
    config::{self, Config, ProjectConfig},
    state::{agent::Provider, project::Project},
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
    pub name_cursor:   usize,  // char-count cursor in name field
    pub path_cursor:   usize,  // char-count cursor in path field
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

/// Convert a char-count cursor position to a byte offset.
fn char_to_byte(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

pub enum AppModal {
    NewProject(NewProjectForm),
    Help,
}

// ─── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub projects:    Vec<Project>,
    pub sidebar_sel: SidebarSelection,
    pub modal:       Option<AppModal>,
    pub config_path: String,
    pub should_quit: bool,
}

impl App {
    pub fn from_config(cfg: &Config, config_path: &str) -> Self {
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
            modal:       None,
            config_path: config_path.to_string(),
            should_quit: false,
        }
    }

    pub fn active_project_index(&self) -> Option<usize> {
        match self.sidebar_sel {
            SidebarSelection::Project(i) => Some(i),
            SidebarSelection::AddButton  => None,
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
        match key.code {
            KeyCode::Char('?') => {
                self.modal = Some(AppModal::Help);
            }
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }

            // Project navigation — includes the "+" slot at index projects.len()
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
            }

            // Open modal when "+" is selected
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let SidebarSelection::AddButton = self.sidebar_sel {
                    self.modal = Some(AppModal::NewProject(NewProjectForm::default()));
                }
            }

            // Tab navigation — only when a real project is selected
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                if let Some(i) = self.active_project_index() {
                    if let Some(p) = self.projects.get_mut(i) {
                        if !p.tabs.is_empty() {
                            p.active_tab = (p.active_tab + 1) % p.tabs.len();
                        }
                    }
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
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

    // ── Modal mode ─────────────────────────────────────────────────────────────

    fn handle_modal_key(&mut self, key: KeyEvent) {
        // Help overlay: any key closes it
        if matches!(self.modal, Some(AppModal::Help)) {
            self.modal = None;
            return;
        }

        // Use a flag so the borrow of self.modal is released before confirm_new_project.
        let mut do_confirm = false;

        {
            let Some(AppModal::NewProject(ref mut form)) = self.modal else { return };

            match key.code {
                KeyCode::Esc => {
                    self.modal = None;
                    return;
                }
                KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                    form.next_field();
                }
                KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
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
                KeyCode::Char(c) => {
                    form.insert_char(c);
                }
                KeyCode::Backspace => {
                    form.delete_char();
                }
                _ => {}
            }
        } // borrow of self.modal released here

        if do_confirm {
            self.confirm_new_project();
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

        // Persist to config file
        let cfg_path = self.config_path.clone();
        match config::load(&cfg_path) {
            Ok(mut cfg) => {
                cfg.project.push(ProjectConfig {
                    name,
                    provider: provider_str,
                    path,
                });
                if let Err(e) = config::save(&cfg, &cfg_path) {
                    tracing::error!("save config: {e}");
                }
            }
            Err(_) => {
                // config.toml doesn't exist yet — create a minimal one
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
}
