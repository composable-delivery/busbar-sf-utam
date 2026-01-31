//! UTAM Compiler
//!
//! Transforms UTAM JSON page object definitions into Rust source code.
//!
//! **Note**: Core compiler functionality is not yet fully implemented.
//! The parser and code generator modules are currently stubs.
//!
//! # Example
//!
//! ```rust,no_run
//! use utam_compiler::validator::SchemaValidator;
//!
//! let json = r#"{"root": true, "selector": {"css": ".button"}}"#;
//! let validator = SchemaValidator::new().unwrap();
//! validator.validate_str(json).expect("Invalid UTAM JSON");
//! ```

mod ast;
mod codegen;
pub mod error;
mod parser;
pub mod validator;

pub use error::{CompilerError, CompilerResult, ValidationError};
pub use validator::SchemaValidator;

// TODO: Re-enable once modules are implemented
// pub use ast::*;
// pub use codegen::generate;
// pub use parser::parse;

// /// Compile UTAM JSON to Rust source code
// pub fn compile(json: &str) -> CompilerResult<String> {
//     let ast = parse(json)?;
//     validate(&ast)?;
//     let code = generate(&ast)?;
//     Ok(code)
// }
