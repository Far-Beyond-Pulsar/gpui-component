//! File synchronization system for multiplayer collaboration
//!
//! This module provides the core infrastructure for syncing project files
//! between multiple clients in a collaborative session.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use std::fs;
use std::io::Read;

/// Represents a file in the project tree
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileNode {
    /// Relative path from project root
    pub path: PathBuf,
    /// SHA-256 hash of file contents
    pub hash: String,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp (Unix epoch)
    pub modified: u64,
    /// Whether this is a directory
    pub is_dir: bool,
}

/// Represents the entire project file tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTree {
    /// Root directory path
    pub root: PathBuf,
    /// Map of relative path -> FileNode
    pub files: HashMap<PathBuf, FileNode>,
}

/// Represents the difference between two project trees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeDiff {
    /// Files that exist on remote but not locally (need to download)
    pub added: Vec<FileNode>,
    /// Files that have different hashes (need to update)
    pub modified: Vec<FileNode>,
    /// Files that exist locally but not on remote (need to delete)
    pub deleted: Vec<PathBuf>,
}

impl FileNode {
    /// Create a FileNode from a file path
    pub fn from_path(path: &Path, root: &Path) -> std::io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let is_dir = metadata.is_dir();

        let relative_path = path.strip_prefix(root)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            .to_path_buf();

        let hash = if !is_dir {
            Self::hash_file(path)?
        } else {
            String::new()
        };

        Ok(Self {
            path: relative_path,
            hash,
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
            is_dir,
        })
    }

    /// Compute SHA-256 hash of a file
    fn hash_file(path: &Path) -> std::io::Result<String> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

impl ProjectTree {
    /// Scan a directory and build a project tree
    pub fn from_directory(root: &Path) -> std::io::Result<Self> {
        let mut files = HashMap::new();
        Self::scan_directory(root, root, &mut files)?;

        Ok(Self {
            root: root.to_path_buf(),
            files,
        })
    }

    /// Recursively scan a directory
    fn scan_directory(
        dir: &Path,
        root: &Path,
        files: &mut HashMap<PathBuf, FileNode>,
    ) -> std::io::Result<()> {
        // Skip common directories that shouldn't be synced
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if Self::should_ignore(dir_name) {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip ignored files/directories
            if Self::should_ignore(file_name) {
                continue;
            }

            let node = FileNode::from_path(&path, root)?;
            files.insert(node.path.clone(), node);

            // Recurse into directories
            if path.is_dir() {
                Self::scan_directory(&path, root, files)?;
            }
        }

        Ok(())
    }

    /// Check if a file/directory should be ignored
    fn should_ignore(name: &str) -> bool {
        matches!(
            name,
            ".git" | ".svn" | ".hg" |
            "node_modules" | "target" | "dist" | "build" |
            ".vscode" | ".idea" | ".vs" |
            "*.log" | "*.tmp" | ".DS_Store" | "Thumbs.db"
        ) || name.starts_with('.')
    }

    /// Compute the difference between this tree and another (remote) tree
    pub fn diff(&self, remote: &ProjectTree) -> TreeDiff {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        // Find added and modified files
        for (path, remote_node) in &remote.files {
            if let Some(local_node) = self.files.get(path) {
                // File exists locally - check if modified
                if !remote_node.is_dir && local_node.hash != remote_node.hash {
                    modified.push(remote_node.clone());
                }
            } else {
                // File doesn't exist locally - needs to be added
                added.push(remote_node.clone());
            }
        }

        // Find deleted files
        for path in self.files.keys() {
            if !remote.files.contains_key(path) {
                deleted.push(path.clone());
            }
        }

        TreeDiff {
            added,
            modified,
            deleted,
        }
    }

    /// Get total number of files (excluding directories)
    pub fn file_count(&self) -> usize {
        self.files.values().filter(|n| !n.is_dir).count()
    }

    /// Get total size of all files in bytes
    pub fn total_size(&self) -> u64 {
        self.files.values().filter(|n| !n.is_dir).map(|n| n.size).sum()
    }
}

impl TreeDiff {
    /// Check if there are any differences
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.modified.is_empty() || !self.deleted.is_empty()
    }

    /// Get total number of changes
    pub fn change_count(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }

    /// Format a human-readable summary
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.added.is_empty() {
            parts.push(format!("{} added", self.added.len()));
        }
        if !self.modified.is_empty() {
            parts.push(format!("{} modified", self.modified.len()));
        }
        if !self.deleted.is_empty() {
            parts.push(format!("{} deleted", self.deleted.len()));
        }

        if parts.is_empty() {
            "No changes".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_node_hash() {
        // Test hashing a known file would go here
    }

    #[test]
    fn test_project_tree_diff() {
        // Test diff algorithm would go here
    }
}
