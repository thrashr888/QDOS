//! Directory Map State Types

use std::fs;
use std::path::PathBuf;

/// Directory tree node for Directory Map
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub expanded: bool,
    pub children: Vec<DirTreeNode>,
    pub depth: usize,
}

impl DirTreeNode {
    pub fn new(name: String, path: PathBuf, depth: usize) -> Self {
        Self {
            name,
            path,
            expanded: false,
            children: Vec::new(),
            depth,
        }
    }

    pub fn load_children(&mut self) {
        if !self.children.is_empty() {
            return;
        }
        if let Ok(entries) = fs::read_dir(&self.path) {
            let mut dirs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .map(|e| {
                    DirTreeNode::new(
                        e.file_name().to_string_lossy().to_string(),
                        e.path(),
                        self.depth + 1,
                    )
                })
                .collect();
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            self.children = dirs;
        }
    }
}

/// Directory Map state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirMapState {
    pub root: DirTreeNode,
    pub selected_index: usize,
    pub flat_list: Vec<(PathBuf, usize, bool, bool)>, // path, depth, expanded, has_children
    pub input_mode: Option<String>,
    pub input_buffer: String,
    pub confirm_delete: Option<PathBuf>,
}

impl DirMapState {
    pub fn new(start_path: &PathBuf) -> Self {
        let root_path = if let Some(root) = start_path.ancestors().last() {
            root.to_path_buf()
        } else {
            start_path.clone()
        };

        let mut root = DirTreeNode::new(root_path.to_string_lossy().to_string(), root_path, 0);
        root.expanded = true;
        root.load_children();

        let mut state = Self {
            root,
            selected_index: 0,
            flat_list: Vec::new(),
            input_mode: None,
            input_buffer: String::new(),
            confirm_delete: None,
        };
        state.expand_to_path(start_path);
        state.rebuild_flat_list();

        if let Some(idx) = state
            .flat_list
            .iter()
            .position(|(p, _, _, _)| p == start_path)
        {
            state.selected_index = idx;
        }

        state
    }

    fn expand_to_path(&mut self, target: &PathBuf) {
        let ancestors: Vec<_> = target.ancestors().collect();
        for ancestor in ancestors.into_iter().rev() {
            self.expand_path_in_tree(&self.root.clone(), &ancestor.to_path_buf());
        }
    }

    fn expand_path_in_tree(&mut self, _node: &DirTreeNode, target: &PathBuf) {
        fn expand_recursive(node: &mut DirTreeNode, target: &PathBuf) {
            if target.starts_with(&node.path) {
                node.expanded = true;
                node.load_children();
                for child in &mut node.children {
                    expand_recursive(child, target);
                }
            }
        }
        expand_recursive(&mut self.root, target);
    }

    pub fn rebuild_flat_list(&mut self) {
        self.flat_list.clear();
        fn flatten(node: &DirTreeNode, list: &mut Vec<(PathBuf, usize, bool, bool)>) {
            let has_children = !node.children.is_empty() || {
                fs::read_dir(&node.path)
                    .map(|entries| {
                        entries.filter_map(|e| e.ok()).any(|e| {
                            e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.')
                        })
                    })
                    .unwrap_or(false)
            };
            list.push((node.path.clone(), node.depth, node.expanded, has_children));
            if node.expanded {
                for child in &node.children {
                    flatten(child, list);
                }
            }
        }
        flatten(&self.root, &mut self.flat_list);
    }

    pub fn toggle_expand(&mut self, index: usize) {
        if index >= self.flat_list.len() {
            return;
        }
        let (path, _, expanded, _) = &self.flat_list[index];
        let path = path.clone();
        let expanded = *expanded;

        fn toggle_in_tree(node: &mut DirTreeNode, target: &PathBuf, expand: bool) -> bool {
            if node.path == *target {
                node.expanded = expand;
                if expand {
                    node.load_children();
                }
                return true;
            }
            for child in &mut node.children {
                if toggle_in_tree(child, target, expand) {
                    return true;
                }
            }
            false
        }

        toggle_in_tree(&mut self.root, &path, !expanded);
        self.rebuild_flat_list();
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.flat_list
            .get(self.selected_index)
            .map(|(p, _, _, _)| p.clone())
    }
}
