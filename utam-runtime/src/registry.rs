//! Page object registry — discovery, caching, and lookup of UTAM JSON definitions.
//!
//! The registry scans directories for `.utam.json` files, parses them into ASTs,
//! and caches them for fast lookup. This enables agents to discover what page
//! objects are available and load them by name.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use utam_compiler::ast::PageObjectAst;

use crate::error::{RuntimeError, RuntimeResult};

/// Registry for discovering and caching UTAM page object definitions.
///
/// # Example
///
/// ```rust,ignore
/// let mut registry = PageObjectRegistry::new();
/// registry.add_search_path("./salesforce-pageobjects");
/// registry.scan()?;
///
/// let ast = registry.get("helpers/login")?;
/// let page = DynamicPageObject::load(driver, ast).await?;
/// ```
#[derive(Debug)]
pub struct PageObjectRegistry {
    /// Parsed ASTs indexed by qualified name (e.g. "global/header")
    cache: RwLock<HashMap<String, PageObjectAst>>,
    /// Directories to search for .utam.json files
    search_paths: Vec<PathBuf>,
}

impl PageObjectRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self { cache: RwLock::new(HashMap::new()), search_paths: Vec::new() }
    }

    /// Add a directory to search for `.utam.json` files
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Scan all search paths and populate the cache.
    ///
    /// Files are indexed by their relative path without the `.utam.json`
    /// suffix (e.g. `helpers/login`).
    pub fn scan(&self) -> RuntimeResult<usize> {
        let mut cache = self.cache.write().unwrap();
        let mut count = 0;

        for base in &self.search_paths {
            if !base.exists() {
                continue;
            }
            for entry in walk_utam_files(base) {
                let content = match std::fs::read_to_string(&entry) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let ast: PageObjectAst = match serde_json::from_str(&content) {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                // Derive the name from the relative path
                let name = entry
                    .strip_prefix(base)
                    .unwrap_or(&entry)
                    .to_string_lossy()
                    .replace('\\', "/")
                    .trim_end_matches(".utam.json")
                    .to_string();
                cache.insert(name, ast);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Register a page object from a JSON string
    pub fn register_json(&self, name: &str, json: &str) -> RuntimeResult<()> {
        let ast: PageObjectAst = serde_json::from_str(json)?;
        self.cache.write().unwrap().insert(name.to_string(), ast);
        Ok(())
    }

    /// Register a pre-parsed AST directly
    pub fn register(&self, name: &str, ast: PageObjectAst) {
        self.cache.write().unwrap().insert(name.to_string(), ast);
    }

    /// Get a page object AST by name
    pub fn get(&self, name: &str) -> RuntimeResult<PageObjectAst> {
        self.cache
            .read()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::PageObjectNotFound { name: name.to_string() })
    }

    /// List all available page object names
    pub fn list(&self) -> Vec<String> {
        let cache = self.cache.read().unwrap();
        let mut names: Vec<String> = cache.keys().cloned().collect();
        names.sort();
        names
    }

    /// Search for page objects whose name contains the query string
    pub fn search(&self, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let cache = self.cache.read().unwrap();
        let mut matches: Vec<String> =
            cache.keys().filter(|k| k.to_lowercase().contains(&query_lower)).cloned().collect();
        matches.sort();
        matches
    }

    /// How many page objects are loaded
    pub fn len(&self) -> usize {
        self.cache.read().unwrap().len()
    }

    /// Whether the registry is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PageObjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively find all .utam.json files under a directory.
fn walk_utam_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_utam_files(&path));
            } else if path.to_string_lossy().ends_with(".utam.json") {
                files.push(path);
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_register_and_get() {
        let registry = PageObjectRegistry::new();
        let json = r#"{ "root": true, "selector": { "css": ".page" } }"#;
        registry.register_json("test/page", json).unwrap();

        let ast = registry.get("test/page").unwrap();
        assert!(ast.root);
    }

    #[test]
    fn test_registry_not_found() {
        let registry = PageObjectRegistry::new();
        let result = registry.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_list_and_search() {
        let registry = PageObjectRegistry::new();
        registry
            .register_json("helpers/login", r#"{ "root": true, "selector": { "css": ".login" } }"#)
            .unwrap();
        registry
            .register_json("global/header", r#"{ "root": true, "selector": { "css": ".header" } }"#)
            .unwrap();

        let all = registry.list();
        assert_eq!(all.len(), 2);

        let found = registry.search("login");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], "helpers/login");

        let found = registry.search("HEADER");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_registry_scan_salesforce_pageobjects() {
        let mut registry = PageObjectRegistry::new();
        let sf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../salesforce-pageobjects");
        if sf_path.exists() {
            registry.add_search_path(&sf_path);
            let count = registry.scan().unwrap();
            assert!(count > 1000, "Expected 1000+ page objects, got {count}");

            // Verify we can find specific ones
            let login_matches = registry.search("login");
            assert!(!login_matches.is_empty(), "Should find login page objects");

            let header_matches = registry.search("header");
            assert!(!header_matches.is_empty(), "Should find header page objects");
        }
    }

    #[test]
    fn test_registry_len() {
        let registry = PageObjectRegistry::new();
        assert!(registry.is_empty());
        registry.register_json("test", r#"{ "root": true, "selector": { "css": ".t" } }"#).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }
}
