use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileTreeEntry {
    pub name:     String,
    pub path:     PathBuf,
    pub is_dir:   bool,
    pub depth:    usize,
    pub expanded: bool,
}

pub struct FileTreeState {
    pub entries:  Vec<FileTreeEntry>,
    pub selected: usize,
}

impl FileTreeState {
    pub fn new(root_path: &str) -> Self {
        let mut s = Self { entries: Vec::new(), selected: 0 };
        s.reload(root_path);
        s
    }

    pub fn reload(&mut self, root_path: &str) {
        self.entries = read_dir_entries(Path::new(root_path), 0);
        self.selected = 0;
    }

    /// Expand or collapse the selected directory entry.
    pub fn toggle_selected(&mut self) {
        let Some(entry) = self.entries.get(self.selected) else { return };
        if !entry.is_dir { return; }

        if entry.expanded {
            // Collapse: remove all descendants (entries with depth > this one's)
            let depth = entry.depth;
            self.entries[self.selected].expanded = false;
            let rm_start = self.selected + 1;
            let rm_end = self.entries[rm_start..]
                .iter()
                .position(|e| e.depth <= depth)
                .map(|p| rm_start + p)
                .unwrap_or(self.entries.len());
            self.entries.drain(rm_start..rm_end);
        } else {
            // Expand: load children and splice them in
            let path  = self.entries[self.selected].path.clone();
            let depth = self.entries[self.selected].depth;
            self.entries[self.selected].expanded = true;
            let children = read_dir_entries(&path, depth + 1);
            let at = self.selected + 1;
            for (i, child) in children.into_iter().enumerate() {
                self.entries.insert(at + i, child);
            }
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn move_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_bottom(&mut self) {
        self.selected = self.entries.len().saturating_sub(1);
    }
}

fn read_dir_entries(path: &Path, depth: usize) -> Vec<FileTreeEntry> {
    let Ok(rd) = std::fs::read_dir(path) else { return Vec::new() };

    let mut dirs:  Vec<(String, PathBuf)> = Vec::new();
    let mut files: Vec<(String, PathBuf)> = Vec::new();

    for entry in rd.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }   // skip hidden
        let path = entry.path();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            dirs.push((name, path));
        } else {
            files.push((name, path));
        }
    }

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut entries = Vec::new();
    for (name, path) in dirs  { entries.push(FileTreeEntry { name, path, is_dir: true,  depth, expanded: false }); }
    for (name, path) in files { entries.push(FileTreeEntry { name, path, is_dir: false, depth, expanded: false }); }
    entries
}
