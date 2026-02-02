//! UTAM Compiler
//!
//! Transforms UTAM JSON page object definitions into Rust source code.
//!
//! **Note**: Core compiler functionality is not yet implemented.
//! The parser, validator, and code generator modules are currently stubs.
//!
//! # Planned Example
//!
//! ```rust,ignore
//! use utam_compiler::compile;
//!
//! let json = include_str!("login-form.utam.json");
//! let rust_code = compile(json)?;
//! ```

pub mod ast;
mod codegen;
mod error;
mod parser;
mod validator;

// Re-export AST types for convenience
pub use ast::*;

// TODO: Re-enable once modules are implemented
// pub use codegen::generate;
// pub use error::{CompilerError, CompilerResult};
// pub use parser::parse;
// pub use validator::validate;

// /// Compile UTAM JSON to Rust source code
// pub fn compile(json: &str) -> CompilerResult<String> {
//     let ast = parse(json)?;
//     validate(&ast)?;
//     let code = generate(&ast)?;
//     Ok(code)
// }
