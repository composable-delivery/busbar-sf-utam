//! UTAM Compiler Library
//!
//! Compiler for UTAM page object definitions.

/// Returns the version of the UTAM compiler library.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
