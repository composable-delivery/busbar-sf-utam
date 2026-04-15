# Architecture

## Overview

busbar-sf-utam provides two paths for using UTAM page objects:

1. **Compiled path** — `utam-compiler` transforms JSON into Rust structs
   at build time. Tests import those structs and call methods directly.
   This is the traditional approach (matching Salesforce's Java/JS clients).

2. **Runtime path** — `utam-runtime` interprets JSON at runtime. A caller
   loads a page object definition, discovers its methods, and executes
   them dynamically. No compilation step required. This path was designed
   for AI agents and exploratory testing.

Both paths share `utam-core` for element wrappers, WebDriver traits, wait
utilities, and shadow DOM support.

```
                        ┌────────────────────────┐
                        │   .utam.json files     │
                        │   (1,454 Salesforce +   │
                        │    custom definitions)  │
                        └───────────┬────────────┘
                                    │
                   ┌────────────────┼────────────────┐
                   ▼                                  ▼
          ┌─────────────────┐               ┌─────────────────┐
          │  utam-compiler  │               │  utam-runtime   │
          │  JSON → Rust    │               │  JSON → runtime │
          │  source code    │               │  interpreter    │
          └────────┬────────┘               └────────┬────────┘
                   │                                  │
                   │ generates                        │ interprets
                   ▼                                  ▼
          ┌─────────────────┐               ┌─────────────────┐
          │  Rust structs   │               │ DynamicPageObj  │
          │  impl PageObj   │               │ call_method()   │
          │  impl RootPO    │               │ get_element()   │
          └────────┬────────┘               └────────┬────────┘
                   │                                  │
                   └──────────────┬───────────────────┘
                                  ▼
                         ┌─────────────────┐
                         │   utam-core     │
                         │   (traits,      │
                         │    elements,    │
                         │    wait, shadow)│
                         └────────┬────────┘
                                  │
                                  ▼
                         ┌─────────────────┐
                         │  UtamDriver     │
                         │  (trait)        │
                         ├─────────────────┤
                         │ ThirtyfourDriver│ ← WebDriver/Selenium
                         │ [future] CDP   │ ← chromiumoxide
                         │ [future] PW    │ ← playwright-rs
                         └─────────────────┘
```

## Crate Dependency Graph

```
utam-cli
  └── utam-compiler
        └── (serde, quote, syn, miette, jsonschema)

utam-test
  └── utam-core
        └── thirtyfour

utam-runtime
  ├── utam-core
  └── utam-compiler  (AST types only: PageObjectAst, ElementAst, etc.)
```

## utam-core

The foundation crate. Provides:

- **Traits**: `Actionable`, `Clickable`, `Editable`, `Draggable`,
  `PageObject`, `RootPageObject`
- **Element wrappers**: `BaseElement`, `ClickableElement`,
  `EditableElement`, `DraggableElement`, `FrameElement`, `Container`
- **Shadow DOM**: `ShadowRoot`, `traverse_shadow_path()`
- **Wait utilities**: `wait_for()`, `WaitConfig`
- **Error types**: `UtamError`, `UtamResult`

These types are used by both the compiled and runtime paths.

## utam-compiler

Transforms UTAM JSON into Rust source code. Key components:

- **AST** (`ast.rs`): `PageObjectAst`, `ElementAst`, `MethodAst`,
  `ComposeStatementAst`, `SelectorAst` — all derive `Serialize +
  Deserialize + Clone`, so the runtime can reuse them directly.
- **Code generator** (`codegen.rs`): Produces Rust structs with
  `impl PageObject`, element getters, and compose method bodies.
- **Validator** (`validator.rs`): JSON Schema validation.
- **Utils** (`utils.rs`): `to_snake_case()`, `to_pascal_case()`.

## utam-runtime

The runtime interpreter. Key components:

### Driver Abstraction (`driver.rs`)

Protocol-agnostic traits:

- `UtamDriver` — browser session (navigate, find_element, screenshot, quit)
- `ElementHandle` — element operations (click, send_keys, text, shadow_root)
- `ShadowRootHandle` — queries within shadow DOM
- `Selector` — CSS, AccessibilityId, IosClassChain, AndroidUiAutomator
- `ThirtyfourDriver` — bundled WebDriver adapter

Any browser automation protocol can be plugged in by implementing
`UtamDriver` and `ElementHandle`.

### Dynamic Element (`element.rs`)

- `DynamicElement` — wraps an `ElementHandle` with a declared capability
  level (base/clickable/editable/draggable)
- `ElementRuntime` trait — `execute(action, args)` dispatches action
  name strings to the correct `ElementHandle` methods
- `RuntimeValue` — dynamically-typed values (Null, String, Bool, Number,
  Element, Elements)

### Page Object (`page_object.rs`)

- `DynamicPageObject` — loads an AST, finds the root element, builds a
  flat element index, and executes compose methods
- `PageObjectRuntime` trait — `call_method()`, `get_element()`,
  `method_signatures()`, `element_names()`
- Compose interpreter — walks `ComposeStatementAst` steps: element
  lookups, action dispatch, argument resolution, self-referential calls,
  filter/matcher evaluation, `waitFor` predicate polling
- Selector resolution — substitutes `%s`/`%d` parameters at runtime

### Registry (`registry.rs`)

- `PageObjectRegistry` — discovers `.utam.json` files from directories,
  parses and caches them, provides search/list/get
- Scans 1,454 Salesforce page objects in ~0.3 seconds

## utam-test

Test harness for browser tests:

- `TestHarness` — WebDriver session management, screenshot-on-failure,
  retry logic, page load waits
- `PageObjectAssertions` — trait-based assertions on `BaseElement`
  (visible, hidden, text equals, has class, etc.)
- `ElementAssertion` — fluent builder with configurable timeouts
- `CollectionAssertions` — count, empty, not-empty
- `utam_test!` macro — concise test definitions

## utam-cli

Command-line interface with subcommands:

- `compile` — compile UTAM JSON to Rust (scaffolded)
- `validate` — validate JSON against schema (scaffolded)
- `init` — initialize project configuration (scaffolded)
- `lint` — lint page object definitions (scaffolded)

## Salesforce Page Objects

The `salesforce-pageobjects/` directory contains 1,454 UTAM page object
definitions extracted from the Salesforce platform. They cover 74 modules
including:

- **global/** — header, app nav, app launcher, record home
- **helpers/** — login page
- **lightning/** — Lightning component framework
- **sales/** — Sales Cloud components
- **flow/** — Flow Designer
- **builder/** — App Builder
- And 68 more domains

Compatibility: 96.5% parse rate, 99.8% codegen rate.
