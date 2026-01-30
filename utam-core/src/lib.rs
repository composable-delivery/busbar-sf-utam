//! UTAM Core Library
//!
//! Core functionality for UTAM (UI Test Automation Model) framework.

/// Returns the version of the UTAM core library.
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
