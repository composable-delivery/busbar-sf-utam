//! UTAM Compiler
//!
//! Transforms UTAM JSON page object definitions into Rust source code.
//!
//! # Example
//!
//! ```rust,no_run
//! use utam_compiler::{compile, CodeGenConfig};
//!
//! let json = r#"{"root": true, "selector": {"css": ".button"}}"#;
//! let config = CodeGenConfig::default();
//! let rust_code = compile(json, config).expect("Failed to compile");
//! ```

pub mod ast;
pub mod codegen;
pub mod error;
mod parser;
pub mod utils;
pub mod validator;

pub use codegen::{CodeGenConfig, CodeGenerator};
pub use error::{CompilerError, CompilerResult, SelectError, ValidationError};
pub use validator::SchemaValidator;

// Re-export AST types for convenience
pub use ast::*;

/// Compile UTAM JSON to Rust source code
pub fn compile(json: &str, config: CodeGenConfig) -> CompilerResult<String> {
    // Parse JSON to AST
    let ast: PageObjectAst = serde_json::from_str(json)?;
    
    // Generate code
    let generator = CodeGenerator::new(ast, config);
    generator.generate()
}
