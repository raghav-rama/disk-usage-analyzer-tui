use crate::core::DirEntryInfo;

#[derive(PartialEq, Clone, Copy)]
pub enum SortBy {
    Name,
    Size,
}

impl Default for SortBy {
    fn default() -> Self {
        SortBy::Size
    }
}

pub struct App {
    pub current_node: DirEntryInfo,
    pub stack: Vec<DirEntryInfo>,
    pub sort_by: SortBy,
    pub selected: usize,
}

impl App {
    pub fn new(root: DirEntryInfo) -> Self {
        App {
            current_node: root.clone(),
            stack: vec![root],
            sort_by: SortBy::default(),
            selected: 0,
        }
    }

    pub fn navigate_into(&mut self) -> bool {
        if let Some(selected_entry) = self.current_node.children.get(self.selected) {
            if selected_entry.is_dir && !selected_entry.children.is_empty() {
                let new_node = selected_entry.clone();
                self.stack.push(new_node.clone());
                self.current_node = new_node;
                self.selected = 0;
                return true;
            }
        }
        false
    }

    pub fn navigate_out(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            if let Some(prev_node) = self.stack.last() {
                self.current_node = prev_node.clone();
                // Try to maintain selection position when going back
                if let Some(pos) = self
                    .current_node
                    .children
                    .iter()
                    .position(|c| c.path == self.current_node.path)
                {
                    self.selected = pos.min(self.current_node.children.len().saturating_sub(1));
                }
                return true;
            }
        }
        false
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.current_node.children.is_empty() {
            return;
        }
        let len = self.current_node.children.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
    }

    pub fn toggle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            SortBy::Name => SortBy::Size,
            SortBy::Size => SortBy::Name,
        };
        self.sort_children();
    }

    pub fn sort_children(&mut self) {
        match self.sort_by {
            SortBy::Name => self
                .current_node
                .children
                .sort_by(|a, b| a.path.file_name().cmp(&b.path.file_name())),
            SortBy::Size => self
                .current_node
                .children
                .sort_by(|a, b| b.size.cmp(&a.size)),
        }
    }
}
