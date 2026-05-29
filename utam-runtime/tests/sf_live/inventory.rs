//! Coverage inventory derived from the compiled UTAM JS client.
//!
//! `scripts/build_coverage_inventory.py` mines the `salesforce-pageobjects`
//! npm package's compiled `.d.ts` declarations (method signatures, argument
//! names, nullability) plus the root selectors, classifies each root page
//! object as **standard** (testable in a vanilla scratch org) or **gated**
//! (requires a managed package or an Experience/CMS feature a standard
//! scratch org doesn't have), and writes `coverage-inventory.json`.
//!
//! The harness embeds that artifact so it can scope coverage honestly:
//! discovered-but-out-of-scope objects are recorded as **Skipped with the
//! capability they require** — never silently passed, never failed.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

const INVENTORY_JSON: &str =
    include_str!("../../../salesforce-pageobjects/coverage-inventory.json");

#[derive(Debug, Deserialize)]
struct Inventory {
    #[serde(default)]
    objects: HashMap<String, ObjectEntry>,
}

#[derive(Debug, Deserialize)]
struct ObjectEntry {
    #[serde(default)]
    standard: bool,
    #[serde(default)]
    gated_reason: Option<String>,
}

static INVENTORY: LazyLock<Inventory> = LazyLock::new(|| {
    serde_json::from_str(INVENTORY_JSON).expect(
        "coverage-inventory.json must be valid JSON (regenerate via build_coverage_inventory.py)",
    )
});

/// If `po_name` is out of scope for a standard scratch org, return the
/// capability it requires (e.g. "DevOps Center managed package"). Standard
/// objects — and any object not in the inventory — return `None`.
pub fn gated_reason(po_name: &str) -> Option<&'static str> {
    INVENTORY.objects.get(po_name).filter(|o| !o.standard).and_then(|o| o.gated_reason.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_parses_and_has_gated_objects() {
        assert!(!INVENTORY.objects.is_empty(), "inventory should not be empty");
        let gated = INVENTORY.objects.values().filter(|o| !o.standard).count();
        assert!(gated > 0, "expected some gated objects in the inventory");
    }

    #[test]
    fn devops_center_is_gated_and_header_is_standard() {
        // DevOps Center is a managed package — out of scope for a standard org.
        assert!(gated_reason("devops/center/baseComponent").is_some());
        // The global header is a standard Lightning Experience component.
        assert!(gated_reason("global/header").is_none());
    }
}
