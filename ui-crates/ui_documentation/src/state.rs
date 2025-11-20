use gpui::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocIndex {
    pub crates: Vec<CrateInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub items: HashMap<String, Vec<DocItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocItem {
    pub name: String,
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub enum TreeNode {
    Crate { name: String, expanded: bool },
    Category { name: String, crate_name: String, expanded: bool },
    Item { name: String, path: String, crate_name: String, category: String },
}

pub struct DocumentationState {
    pub doc_index: DocIndex,
    pub tree_items: Vec<TreeNode>,
    pub flat_visible_items: Vec<usize>,
    pub selected_item: Option<String>,
    pub search_query: SharedString,
    pub search_results: Vec<DocItem>,
    pub current_doc_content: SharedString,
}

impl DocumentationState {
    pub fn new(cx: &mut Context<impl Send>) -> Self {
        let docs_data = pulsar_docs::DOCS_DATA
            .iter()
            .find(|(name, _)| *name == "index.json")
            .map(|(_, content)| content);

        let doc_index: DocIndex = if let Some(data) = docs_data {
            serde_json::from_str(data).unwrap_or_else(|_| DocIndex { crates: vec![] })
        } else {
            DocIndex { crates: vec![] }
        };

        let mut state = Self {
            doc_index,
            tree_items: vec![],
            flat_visible_items: vec![],
            selected_item: None,
            search_query: "".into(),
            search_results: vec![],
            current_doc_content: "".into(),
        };

        state.rebuild_tree();
        state
    }

    pub fn rebuild_tree(&mut self) {
        self.tree_items.clear();
        
        for crate_info in &self.doc_index.crates {
            self.tree_items.push(TreeNode::Crate {
                name: crate_info.name.clone(),
                expanded: false,
            });
        }
        
        self.rebuild_flat_list();
    }

    pub fn rebuild_flat_list(&mut self) {
        self.flat_visible_items.clear();
        
        let items = self.tree_items.clone();
        for (idx, node) in items.iter().enumerate() {
            match node {
                TreeNode::Crate { expanded, name } => {
                    self.flat_visible_items.push(idx);
                    if *expanded {
                        self.add_crate_children(name, idx);
                    }
                }
                TreeNode::Category { expanded, crate_name, name } => {
                    if self.is_parent_expanded(crate_name) {
                        self.flat_visible_items.push(idx);
                        if *expanded {
                            self.add_category_children(crate_name, name, idx);
                        }
                    }
                }
                TreeNode::Item { crate_name, category, .. } => {
                    if self.is_category_expanded(crate_name, category) {
                        self.flat_visible_items.push(idx);
                    }
                }
            }
        }
    }

    fn add_crate_children(&mut self, crate_name: &str, crate_idx: usize) {
        if let Some(crate_info) = self.doc_index.crates.iter().find(|c| c.name == crate_name) {
            let mut categories: Vec<String> = crate_info.items.keys().cloned().collect();
            categories.sort();
            
            for category in categories {
                let existing = self.tree_items.iter().position(|node| {
                    matches!(node, TreeNode::Category { name, crate_name: cn, .. } 
                        if name == &category && cn == crate_name)
                });
                
                if existing.is_none() {
                    self.tree_items.insert(crate_idx + 1, TreeNode::Category {
                        name: category,
                        crate_name: crate_name.to_string(),
                        expanded: false,
                    });
                }
            }
        }
    }

    fn add_category_children(&mut self, crate_name: &str, category: &str, cat_idx: usize) {
        if let Some(crate_info) = self.doc_index.crates.iter().find(|c| c.name == crate_name) {
            if let Some(items) = crate_info.items.get(category) {
                let mut sorted_items = items.clone();
                sorted_items.sort_by(|a, b| a.name.cmp(&b.name));
                
                for item in sorted_items {
                    let existing = self.tree_items.iter().position(|node| {
                        matches!(node, TreeNode::Item { name, .. } if name == &item.name)
                    });
                    
                    if existing.is_none() {
                        self.tree_items.insert(cat_idx + 1, TreeNode::Item {
                            name: item.name,
                            path: item.path,
                            crate_name: crate_name.to_string(),
                            category: category.to_string(),
                        });
                    }
                }
            }
        }
    }

    fn is_parent_expanded(&self, crate_name: &str) -> bool {
        self.tree_items.iter().any(|node| {
            matches!(node, TreeNode::Crate { name, expanded } 
                if name == crate_name && *expanded)
        })
    }

    fn is_category_expanded(&self, crate_name: &str, category: &str) -> bool {
        self.tree_items.iter().any(|node| {
            matches!(node, TreeNode::Category { name, crate_name: cn, expanded } 
                if name == category && cn == crate_name && *expanded)
        })
    }

    pub fn toggle_node(&mut self, idx: usize, cx: &mut Context<impl Send>) {
        if idx >= self.tree_items.len() {
            return;
        }

        let node = self.tree_items[idx].clone();
        match node {
            TreeNode::Crate { .. } => {
                if let TreeNode::Crate { ref mut expanded, .. } = self.tree_items[idx] {
                    *expanded = !*expanded;
                }
            }
            TreeNode::Category { .. } => {
                if let TreeNode::Category { ref mut expanded, .. } = self.tree_items[idx] {
                    *expanded = !*expanded;
                }
            }
            TreeNode::Item { path, .. } => {
                self.load_document(&path, cx);
                return;
            }
        }

        self.rebuild_flat_list();
    }

    pub fn load_document(&mut self, path: &str, _cx: &mut Context<impl Send>) {
        self.selected_item = Some(path.to_string());
        
        let doc_content = pulsar_docs::DOCS_DATA
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, content)| content.to_string())
            .unwrap_or_else(|| "# Documentation not found".to_string());

        self.current_doc_content = doc_content.into();
    }

    pub fn update_search(&mut self, query: String, _cx: &mut Context<impl Send>) {
        self.search_query = query.clone().into();
        
        if query.is_empty() {
            self.search_results.clear();
            return;
        }

        let query_lower = query.to_lowercase();
        self.search_results.clear();

        for crate_info in &self.doc_index.crates {
            for items in crate_info.items.values() {
                for item in items {
                    if item.name.to_lowercase().contains(&query_lower) {
                        self.search_results.push(item.clone());
                    }
                }
            }
        }
    }
}
