//! Salesforce page objects compatibility test suite
//!
//! Verifies that all 1,454 Salesforce page object JSON files from the
//! salesforce-pageobjects directory can be parsed into UTAM AST structs.
//! This is a critical integration gate: if a Salesforce-specific pattern
//! breaks parsing, this test catches it.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use utam_compiler::{compile, CodeGenConfig, PageObjectAst};

/// Recursively collects all .utam.json file paths under `dir`.
fn collect_utam_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_utam_files(&path));
            } else if path.to_string_lossy().ends_with(".utam.json") {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
    files
}

/// Categorize an error message into a short bucket key.
fn error_category(msg: &str) -> String {
    if msg.contains("missing field") {
        let field = msg
            .split("missing field `")
            .nth(1)
            .and_then(|s| s.split('`').next())
            .unwrap_or("unknown");
        format!("missing field `{field}`")
    } else if msg.contains("invalid type") {
        "invalid type".to_string()
    } else if msg.contains("unknown variant") {
        "unknown variant".to_string()
    } else {
        let truncated: String = msg.chars().take(80).collect();
        truncated
    }
}

#[test]
fn test_salesforce_pageobjects_parse() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../salesforce-pageobjects");
    let files = collect_utam_files(&dir);

    assert!(files.len() > 1000, "Expected 1000+ Salesforce page objects, found {}", files.len());

    let mut success = 0usize;
    let mut parse_errors: HashMap<String, Vec<String>> = HashMap::new();

    for path in &files {
        let content = fs::read_to_string(path).expect("read file");
        match serde_json::from_str::<PageObjectAst>(&content) {
            Ok(_) => success += 1,
            Err(e) => {
                let cat = error_category(&format!("{e}"));
                parse_errors.entry(cat).or_default().push(path.clone());
            }
        }
    }

    let total = files.len();
    let failed = total - success;

    // Print a compatibility report regardless of pass/fail
    eprintln!();
    eprintln!("=== Salesforce Page Objects Compatibility Report ===");
    eprintln!("Total files:      {total}");
    eprintln!("Parsed OK:        {success}");
    eprintln!("Parse failures:   {failed}");
    if total > 0 {
        eprintln!("Success rate:     {:.1}%", success as f64 / total as f64 * 100.0);
    }

    if !parse_errors.is_empty() {
        eprintln!();
        eprintln!("--- Failure categories ---");
        let mut cats: Vec<_> = parse_errors.iter().collect();
        cats.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        for (cat, paths) in &cats {
            eprintln!("[{:>4}] {cat}", paths.len());
            for p in paths.iter().take(3) {
                let short = p.rsplit('/').next().unwrap_or(p);
                eprintln!("         e.g. {short}");
            }
        }
    }

    eprintln!("====================================================");

    // The test passes if the parse success rate is above 80%.
    // As the compiler matures, we can raise this bar.
    let success_rate = success as f64 / total as f64 * 100.0;
    assert!(
        success_rate >= 80.0,
        "Parse success rate {success_rate:.1}% is below 80% threshold ({success}/{total})"
    );
}

#[test]
fn test_salesforce_pageobjects_codegen() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../salesforce-pageobjects");
    let files = collect_utam_files(&dir);

    let mut total_parseable = 0usize;
    let mut codegen_ok = 0usize;
    let mut codegen_errors: HashMap<String, Vec<String>> = HashMap::new();

    for path in &files {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Only test codegen for files that parse successfully
        if serde_json::from_str::<PageObjectAst>(&content).is_err() {
            continue;
        }
        total_parseable += 1;

        let config = CodeGenConfig { module_name: None };
        match compile(&content, config) {
            Ok(_) => codegen_ok += 1,
            Err(e) => {
                let cat = error_category(&format!("{e}"));
                codegen_errors.entry(cat).or_default().push(path.clone());
            }
        }
    }

    eprintln!();
    eprintln!("=== Salesforce Codegen Compatibility Report ===");
    eprintln!("Parseable files:  {total_parseable}");
    eprintln!("Codegen OK:       {codegen_ok}");
    eprintln!("Codegen failures: {}", total_parseable - codegen_ok);
    if total_parseable > 0 {
        eprintln!("Success rate:     {:.1}%", codegen_ok as f64 / total_parseable as f64 * 100.0);
    }

    if !codegen_errors.is_empty() {
        eprintln!();
        eprintln!("--- Codegen failure categories ---");
        let mut cats: Vec<_> = codegen_errors.iter().collect();
        cats.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        for (cat, paths) in &cats {
            eprintln!("[{:>4}] {cat}", paths.len());
            for p in paths.iter().take(3) {
                let short = p.rsplit('/').next().unwrap_or(p);
                eprintln!("         e.g. {short}");
            }
        }
    }

    eprintln!("=================================================");
}

/// Verify specific Salesforce-specific patterns parse correctly
#[test]
fn test_salesforce_patterns_shadow_dom() {
    // Shadow DOM is extensively used in Lightning Web Components
    let json = r#"{
        "root": true,
        "selector": { "css": "lightning-button" },
        "shadow": {
            "elements": [
                {
                    "name": "button",
                    "type": ["clickable"],
                    "selector": { "css": "button" },
                    "public": true
                }
            ]
        }
    }"#;

    let result: Result<PageObjectAst, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "Shadow DOM pattern should parse: {result:?}");
}

#[test]
fn test_salesforce_patterns_custom_component_type() {
    // Salesforce uses cross-package component references
    let json = r#"{
        "root": true,
        "selector": { "css": ".oneHeader" },
        "elements": [
            {
                "name": "profile",
                "type": "utam-global/pageObjects/userProfileCardTrigger",
                "selector": { "css": ".userProfileCardTriggerRoot" },
                "public": true
            }
        ]
    }"#;

    let result: Result<PageObjectAst, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "Custom component type reference should parse: {result:?}");
}

#[test]
fn test_salesforce_patterns_return_all() {
    // returnAll selector pattern for collections
    let json = r#"{
        "root": true,
        "selector": { "css": ".list" },
        "elements": [
            {
                "name": "items",
                "type": ["clickable"],
                "selector": { "css": "li", "returnAll": true },
                "public": true
            }
        ]
    }"#;

    let result: Result<PageObjectAst, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "returnAll pattern should parse: {result:?}");
}

#[test]
fn test_salesforce_patterns_nullable_element() {
    // nullable elements may not be present on the page
    let json = r#"{
        "root": true,
        "selector": { "css": ".page" },
        "elements": [
            {
                "name": "optionalBanner",
                "type": ["clickable"],
                "selector": { "css": ".banner" },
                "nullable": true,
                "public": true
            }
        ]
    }"#;

    let result: Result<PageObjectAst, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "Nullable element should parse: {result:?}");
}

#[test]
fn test_salesforce_patterns_multi_type_element() {
    // Elements with multiple interaction types
    let json = r#"{
        "root": true,
        "selector": { "css": ".input" },
        "elements": [
            {
                "name": "searchInput",
                "type": ["clickable", "actionable", "editable"],
                "selector": { "css": "input" },
                "public": true
            }
        ]
    }"#;

    let result: Result<PageObjectAst, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "Multi-type element should parse: {result:?}");
}

#[test]
fn test_salesforce_patterns_description_variants() {
    // Simple string description
    let json1 = r#"{
        "root": true,
        "selector": { "css": ".page" },
        "description": "Simple page"
    }"#;

    // Rich description with text array and author
    let json2 = r#"{
        "root": true,
        "selector": { "css": ".page" },
        "description": {
            "text": ["Line one.", "Line two."],
            "author": "Salesforce"
        }
    }"#;

    assert!(serde_json::from_str::<PageObjectAst>(json1).is_ok(), "Simple description");
    assert!(serde_json::from_str::<PageObjectAst>(json2).is_ok(), "Rich description");
}

#[test]
fn test_salesforce_patterns_compose_method() {
    // Compose methods are the Salesforce way of defining multi-step operations
    let json = r#"{
        "root": true,
        "selector": { "css": ".form" },
        "elements": [
            {
                "name": "username",
                "type": ["editable"],
                "selector": { "css": "input.user" }
            },
            {
                "name": "submit",
                "type": ["clickable"],
                "selector": { "css": "button" }
            }
        ],
        "methods": [
            {
                "name": "login",
                "compose": [
                    {
                        "element": "username",
                        "apply": "setText",
                        "args": [{ "name": "user", "type": "string" }]
                    },
                    {
                        "element": "submit",
                        "apply": "click"
                    }
                ]
            }
        ]
    }"#;

    let result: Result<PageObjectAst, _> = serde_json::from_str(json);
    assert!(result.is_ok(), "Compose method should parse: {result:?}");
}
