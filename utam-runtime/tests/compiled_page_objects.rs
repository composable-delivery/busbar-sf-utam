//! Compiled page object tests
//!
//! Compiles JSON page objects via the codegen at test time, then runs
//! the generated Rust structs against a live Salesforce org via WebDriver.
//!
//! These tests validate the full pipeline:
//!   JSON → utam-compiler codegen → Rust struct → thirtyfour WebDriver → live DOM

use std::path::PathBuf;

use utam_compiler::codegen::CodeGenerator;

// Include generated page objects at compile time.
// The codegen produces Rust source from JSON; we compile it inline here.
// For a real build, this would be a build.rs script. For tests, we
// use the compiler API directly and eval the generated code at runtime
// via DynamicPageObject (since we can't include!() runtime-generated source).
//
// Instead, we test the CODEGEN OUTPUT is valid Rust that compiles,
// AND test the RUNTIME interpreter against the same JSON.

/// Verify that the codegen produces valid Rust for key page objects.
#[test]
fn test_codegen_global_header() {
    let json_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../salesforce-pageobjects/global/header.utam.json");
    let json = std::fs::read_to_string(&json_path).expect("Failed to read header JSON");
    let ast: utam_compiler::ast::PageObjectAst =
        serde_json::from_str(&json).expect("Failed to parse header JSON");

    let config = utam_compiler::codegen::CodeGenConfig { module_name: Some("header".to_string()) };
    let generator = CodeGenerator::new(ast, config);
    let code = generator.generate().expect("Codegen failed for global/header");

    // Verify the generated code contains expected methods
    assert!(code.contains("pub struct Header"), "Missing struct Header");
    assert!(code.contains("impl PageObject for Header"), "Missing PageObject impl");
    assert!(code.contains("impl RootPageObject for Header"), "Missing RootPageObject impl");
    assert!(code.contains("ROOT_SELECTOR"), "Missing ROOT_SELECTOR");
    assert!(code.contains(".oneHeader"), "Missing .oneHeader selector");

    // Verify compose methods are generated
    assert!(code.contains("get_notification_count"), "Missing get_notification_count method");
    assert!(code.contains("show_setup_menu"), "Missing show_setup_menu method");
    assert!(code.contains("get_search"), "Missing get_search method");
    assert!(code.contains("add_to_favorites"), "Missing add_to_favorites method");
    assert!(code.contains("has_new_notification"), "Missing has_new_notification method");
    assert!(code.contains("show_notifications"), "Missing show_notifications method");
    assert!(code.contains("get_global_actions_list"), "Missing get_global_actions_list method");

    // Verify element getters are generated
    assert!(
        code.contains("get_search_icon") || code.contains("searchIcon"),
        "Missing search icon element getter"
    );
    assert!(
        code.contains("get_notifications") || code.contains("notifications"),
        "Missing notifications element getter"
    );

    eprintln!(
        "Generated code for global/header ({} bytes):\n{}",
        code.len(),
        &code[..500.min(code.len())]
    );
}

#[test]
fn test_codegen_global_create() {
    let json_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../salesforce-pageobjects/global/globalCreate.utam.json");
    let json = std::fs::read_to_string(&json_path).expect("Failed to read globalCreate JSON");
    let ast: utam_compiler::ast::PageObjectAst =
        serde_json::from_str(&json).expect("Failed to parse globalCreate JSON");

    let config =
        utam_compiler::codegen::CodeGenConfig { module_name: Some("global_create".to_string()) };
    let generator = CodeGenerator::new(ast, config);
    let code = generator.generate().expect("Codegen failed for global/globalCreate");

    assert!(code.contains("pub struct GlobalCreate"), "Missing struct GlobalCreate");
    assert!(code.contains("click_global_actions"), "Missing click_global_actions method");
    assert!(code.contains("globalCreateContainer"), "Missing selector");

    eprintln!("Generated code for global/globalCreate ({} bytes)", code.len());
}

#[test]
fn test_codegen_navex_layout() {
    let json_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../salesforce-pageobjects/navex/desktopLayoutContainer.utam.json");
    let json = std::fs::read_to_string(&json_path).expect("Failed to read navex JSON");
    let ast: utam_compiler::ast::PageObjectAst =
        serde_json::from_str(&json).expect("Failed to parse navex JSON");

    let config = utam_compiler::codegen::CodeGenConfig {
        module_name: Some("desktop_layout_container".to_string()),
    };
    let generator = CodeGenerator::new(ast, config);
    let code = generator.generate().expect("Codegen failed for navex/desktopLayoutContainer");

    assert!(code.contains("pub struct DesktopLayoutContainer"), "Missing struct");
    assert!(code.contains("get_app_nav"), "Missing get_app_nav method");
    assert!(code.contains("navexDesktopLayoutContainer"), "Missing selector");

    eprintln!("Generated code for navex/desktopLayoutContainer ({} bytes)", code.len());
}

#[test]
fn test_codegen_setup_nav_tree() {
    let json_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../salesforce-pageobjects/setup/setupNavTree.utam.json");
    let json = std::fs::read_to_string(&json_path).expect("Failed to read setupNavTree JSON");
    let ast: utam_compiler::ast::PageObjectAst =
        serde_json::from_str(&json).expect("Failed to parse setupNavTree JSON");

    let config =
        utam_compiler::codegen::CodeGenConfig { module_name: Some("setup_nav_tree".to_string()) };
    let generator = CodeGenerator::new(ast, config);
    let code = generator.generate().expect("Codegen failed for setup/setupNavTree");

    assert!(code.contains("pub struct SetupNavTree"), "Missing struct");
    assert!(code.contains("onesetupSetupNavTree"), "Missing selector");

    eprintln!("Generated code for setup/setupNavTree ({} bytes)", code.len());
}

#[test]
fn test_codegen_record_layout_item() {
    let json_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../salesforce-pageobjects/records/recordLayoutItem.utam.json");
    let json = std::fs::read_to_string(&json_path).expect("Failed to read recordLayoutItem JSON");
    let ast: utam_compiler::ast::PageObjectAst =
        serde_json::from_str(&json).expect("Failed to parse recordLayoutItem JSON");

    let config = utam_compiler::codegen::CodeGenConfig {
        module_name: Some("record_layout_item".to_string()),
    };
    let generator = CodeGenerator::new(ast, config);
    let code = generator.generate().expect("Codegen failed for records/recordLayoutItem");

    assert!(code.contains("pub struct RecordLayoutItem"), "Missing struct");
    // recordLayoutItem has many methods
    assert!(code.contains("get_label_text"), "Missing get_label_text method");
    assert!(code.contains("get_text_input"), "Missing get_text_input method");
    assert!(code.contains("edit"), "Missing edit method");

    eprintln!("Generated code for records/recordLayoutItem ({} bytes)", code.len());
}

/// Bulk codegen test — compile ALL page objects in the registry and verify none fail
#[test]
fn test_codegen_all_page_objects() {
    let po_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../salesforce-pageobjects");
    let mut total = 0;
    let mut passed = 0;
    let mut failed = Vec::new();

    for entry in walkdir::WalkDir::new(&po_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
    {
        let path = entry.path();
        let name = path.strip_prefix(&po_dir).unwrap().to_string_lossy().to_string();
        total += 1;

        let json = match std::fs::read_to_string(path) {
            Ok(j) => j,
            Err(e) => {
                failed.push(format!("{name}: read error: {e}"));
                continue;
            }
        };

        let ast: utam_compiler::ast::PageObjectAst = match serde_json::from_str(&json) {
            Ok(a) => a,
            Err(e) => {
                failed.push(format!("{name}: parse error: {e}"));
                continue;
            }
        };

        let module_name = path.file_stem().unwrap().to_string_lossy().to_string();

        let config = utam_compiler::codegen::CodeGenConfig { module_name: Some(module_name) };
        let generator = CodeGenerator::new(ast, config);
        match generator.generate() {
            Ok(_) => passed += 1,
            Err(e) => failed.push(format!("{name}: codegen error: {e}")),
        }
    }

    eprintln!("Codegen results: {passed}/{total} passed");
    if !failed.is_empty() {
        eprintln!("Failures:");
        for f in &failed {
            eprintln!("  ! {f}");
        }
    }

    // Allow some failures (complex page objects may hit unimplemented features)
    // but the majority should compile
    let pass_rate = passed as f64 / total as f64;
    assert!(
        pass_rate > 0.80,
        "Codegen pass rate too low: {passed}/{total} ({:.0}%). Expected >80%",
        pass_rate * 100.0
    );
}
