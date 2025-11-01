// src/app.rs
use crate::log::InstallEntry;

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Filter,
}

pub struct App {
    pub all_entries: Vec<InstallEntry>,
    pub filtered_entries: Vec<usize>, // indices into all_entries
    pub selected: usize,
    pub input_mode: InputMode,
    pub filter_input: String,
}

impl App {
    pub fn new(entries: Vec<InstallEntry>) -> Self {
        let filtered_entries = (0..entries.len()).collect();
        App {
            all_entries: entries,
            filtered_entries,
            selected: 0,
            input_mode: InputMode::Normal,
            filter_input: String::new(),
        }
    }

    pub fn visible_entries(&self) -> Vec<&InstallEntry> {
        self.filtered_entries
            .iter()
            .filter_map(|&idx| self.all_entries.get(idx))
            .collect()
    }

    pub fn next(&mut self) {
        if !self.filtered_entries.is_empty() {
            self.selected = (self.selected + 1) % self.filtered_entries.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.filtered_entries.is_empty() {
            if self.selected == 0 {
                self.selected = self.filtered_entries.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    pub fn current(&self) -> Option<&InstallEntry> {
        self.filtered_entries
            .get(self.selected)
            .and_then(|&idx| self.all_entries.get(idx))
    }

    pub fn apply_filter(&mut self) {
        let filter = self.filter_input.to_lowercase();
        
        if filter.is_empty() {
            self.filtered_entries = (0..self.all_entries.len()).collect();
        } else {
            self.filtered_entries = self.all_entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    let name = std::path::Path::new(&e.path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    
                    name.to_lowercase().contains(&filter)
                        || e.repo.to_lowercase().contains(&filter)
                        || e.path.to_lowercase().contains(&filter)
                })
                .map(|(i, _)| i)
                .collect();
        }
        
        // Reset selection
        if self.selected >= self.filtered_entries.len() {
            self.selected = if self.filtered_entries.is_empty() {
                0
            } else {
                self.filtered_entries.len() - 1
            };
        }
    }
}

