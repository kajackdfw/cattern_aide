use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatusCode {
    Staged,          // "M " — modified, staged
    Modified,        // " M" — modified in worktree
    StagedModified,  // "MM" — staged and also has unstaged changes
    Added,           // "A " — new file staged
    Deleted,         // "D " or " D"
    Renamed,         // "R "
    Untracked,       // "??"
    Other(String),
}

#[derive(Debug, Clone)]
pub struct GitFileEntry {
    pub path:   String,
    pub status: FileStatusCode,
}

#[derive(Debug)]
pub struct GitStatus {
    pub branch:      String,
    pub files:       Vec<GitFileEntry>,
    pub is_git_repo: bool,
    pub selected:    usize,
}

impl GitStatus {
    pub fn new(project_path: &str) -> Self {
        let mut s = Self { branch: String::new(), files: Vec::new(), is_git_repo: false, selected: 0 };
        s.refresh(project_path);
        s
    }

    pub fn refresh(&mut self, project_path: &str) {
        self.files.clear();
        self.selected = 0;

        let Ok(out) = Command::new("git")
            .args(["-C", project_path, "status", "--porcelain=v1", "-b"])
            .output()
        else {
            self.is_git_repo = false;
            return;
        };

        if !out.status.success() {
            self.is_git_repo = false;
            return;
        }

        self.is_git_repo = true;
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                self.branch = rest.split("...").next().unwrap_or(rest).trim().to_string();
            } else if line.len() >= 3 {
                let xy   = &line[..2];
                let path = line[3..].to_string();
                self.files.push(GitFileEntry { path, status: parse_xy(xy) });
            }
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.files.len() {
            self.selected += 1;
        }
    }
}

fn parse_xy(xy: &str) -> FileStatusCode {
    match xy {
        "M " => FileStatusCode::Staged,
        " M" => FileStatusCode::Modified,
        "MM" => FileStatusCode::StagedModified,
        "A " => FileStatusCode::Added,
        "D " | " D" => FileStatusCode::Deleted,
        "R " => FileStatusCode::Renamed,
        "??" => FileStatusCode::Untracked,
        o    => FileStatusCode::Other(o.to_string()),
    }
}
