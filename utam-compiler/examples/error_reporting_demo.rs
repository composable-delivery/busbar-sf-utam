//! Example demonstrating error reporting with source locations

use miette::{NamedSource, SourceSpan};
use utam_compiler::{CompilerError, ErrorReporter};

fn main() {
    println!("=== UTAM Compiler Error Reporting Demo ===\n");

    // Example 1: Invalid Element Type
    println!("Example 1: Invalid Element Type");
    println!("================================");
    let source1 = r#"{
  "root": true,
  "selector": {"css": ".button"},
  "type": ["unknownType"]
}"#
    .to_string();

    let offset1 = source1.find("unknownType").unwrap();
    let length1 = "unknownType".len();

    let error1 = CompilerError::InvalidElementType {
        src: NamedSource::new("button.utam.json", source1.clone()),
        span: SourceSpan::new(offset1.into(), length1),
    };

    let reporter1 = ErrorReporter::new(source1.clone(), "button.utam.json".to_string());
    reporter1.report(&error1);
    println!();

    // Example 2: Unknown Action Type
    println!("\nExample 2: Unknown Action Type");
    println!("===============================");
    let source2 = r#"{
  "root": true,
  "selector": {"css": ".input"},
  "type": ["invalidAction", "editable"]
}"#
    .to_string();

    let offset2 = source2.find("invalidAction").unwrap();
    let length2 = "invalidAction".len();

    let error2 = CompilerError::UnknownActionType {
        action: "invalidAction".to_string(),
        src: NamedSource::new("input.utam.json", source2.clone()),
        span: SourceSpan::new(offset2.into(), length2),
    };

    let reporter2 = ErrorReporter::new(source2.clone(), "input.utam.json".to_string());
    reporter2.report(&error2);
    println!();

    // Example 3: Selector Parameter Mismatch
    println!("\nExample 3: Selector Parameter Mismatch");
    println!("======================================");
    let source3 = r#"{
  "root": true,
  "selector": {
    "css": "button[data-id='%s'][data-name='%s']",
    "args": [
      {"name": "id", "type": "string"}
    ]
  }
}"#
    .to_string();

    let offset3 = source3.find("\"button[data-id='%s'][data-name='%s']\"").unwrap();
    let length3 = "\"button[data-id='%s'][data-name='%s']\"".len();

    let error3 = CompilerError::SelectorParameterMismatch {
        expected: 2,
        actual: 1,
        src: NamedSource::new("form.utam.json", source3.clone()),
        span: SourceSpan::new(offset3.into(), length3),
    };

    let reporter3 = ErrorReporter::new(source3.clone(), "form.utam.json".to_string());
    reporter3.report(&error3);
    println!();

    // Example 4: JSON Output Format
    println!("\nExample 4: Machine-Readable JSON Output");
    println!("========================================");
    let errors = vec![error1, error2, error3];
    let json_reporter = ErrorReporter::new("".to_string(), "combined.utam.json".to_string());
    let json_output = json_reporter.report_json(&errors);
    println!("{}", json_output);
}
