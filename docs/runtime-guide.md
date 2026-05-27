# Runtime Interpreter Guide

The `utam-runtime` crate lets you load UTAM page object definitions at
runtime and execute their methods dynamically. No compilation step, no
code generation — just JSON in, callable interface out.

## When to Use the Runtime

Use the runtime when you need:

- **Agent-driven testing** — AI agents loading and calling page objects
  during a session without a build step
- **Exploratory testing** — interactively discover and call page object
  methods against a live page
- **Dynamic page objects** — load definitions that aren't known at
  compile time (user-provided, generated on the fly, etc.)
- **Cross-language access** — any language that can call Rust (via FFI,
  HTTP, or an MCP server) can use UTAM page objects

For static test suites where you know the page objects at build time,
the compiled path (`utam-compiler`) generates Rust structs with full
type safety and zero runtime overhead.

## Core Concepts

### PageObjectRegistry

The registry discovers and caches `.utam.json` files:

```rust
use utam_runtime::prelude::*;

let mut registry = PageObjectRegistry::new();
registry.add_search_path("./salesforce-pageobjects");
registry.scan()?;

// How many loaded?
println!("{} page objects available", registry.len());
// → 1403 page objects available

// Search by name
let matches = registry.search("login");
// → ["helpers/login"]

// List everything
let all = registry.list();
```

### DynamicPageObject

Load a page object AST and bind it to a live browser session:

```rust
let ast = registry.get("helpers/login")?;
let page = DynamicPageObject::load(driver, ast).await?;
```

`load()` finds the root element using the page object's selector. If the
page object has `beforeLoad` predicates, it waits for the root element
to appear.

### Introspection

Discover what a page object offers before calling anything:

```rust
// What methods are available?
for method in page.method_signatures() {
    println!("{}({})", method.name,
        method.args.iter()
            .map(|a| format!("{}: {}", a.name, a.arg_type))
            .collect::<Vec<_>>().join(", ")
    );
}
// → login(userNameStr: string, passwordStr: string)
// → loginToHomePage(userNameStr: string, passwordStr: string, partialLandingUrl: string)

// What elements are available?
for name in page.element_names() {
    println!("  - {name}");
}
// → userName, password, submitBtn

// Description
if let Some(desc) = page.description() {
    println!("{desc}");
}
```

### Calling Methods

```rust
use std::collections::HashMap;

let mut args = HashMap::new();
args.insert("userNameStr".into(), RuntimeValue::String("admin@sf.com".into()));
args.insert("passwordStr".into(), RuntimeValue::String("pass123".into()));

let result = page.call_method("login", &args).await?;
```

Arguments are passed as `HashMap<String, RuntimeValue>` where the keys
match the UTAM method argument names.

### Getting Elements

```rust
let element = page.get_element("submitBtn", &HashMap::new()).await?;

// Execute actions on it
use utam_runtime::ElementRuntime;
let text = element.execute("getText", &[]).await?;
let visible = element.execute("isVisible", &[]).await?;
element.execute("click", &[]).await?;
```

## Driver Abstraction

The runtime is decoupled from any specific browser automation protocol
via the `UtamDriver` trait. The bundled adapter uses `thirtyfour`
(WebDriver/Selenium):

```rust
use thirtyfour::prelude::*;
use utam_runtime::ThirtyfourDriver;

let caps = DesiredCapabilities::chrome();
let webdriver = WebDriver::new("http://localhost:4444", caps).await?;
let driver = Box::new(ThirtyfourDriver::new(webdriver));
```

To use a different protocol (CDP, Playwright), implement `UtamDriver`
and `ElementHandle` for your driver. See [architecture.md](architecture.md)
for the trait surface.

## RuntimeValue

All values flowing through the interpreter are dynamically typed:

```rust
enum RuntimeValue {
    Null,
    String(String),
    Bool(bool),
    Number(i64),
    Element(Box<DynamicElement>),
    Elements(Vec<DynamicElement>),
}
```

Extract typed values:

```rust
let text: &str = result.as_str()?;
let flag: bool = result.as_bool()?;
```

## Compose Method Execution

When you call `call_method("login", &args)`, the interpreter walks the
compose statements sequentially:

```json
{
  "compose": [
    { "element": "userName", "apply": "clearAndType",
      "args": [{ "name": "userNameStr", "type": "string" }] },
    { "element": "submitBtn", "apply": "click" }
  ]
}
```

Each step:
1. Resolves the element by name (from root or shadow DOM)
2. Resolves arguments (references to method args, or literals)
3. Dispatches the action to the `ElementHandle`

The interpreter handles:
- **Element + action**: `element.apply(args)`
- **Self-referential calls**: method A calling method B on the same page object
- **Chaining**: using the previous step's result as the next step's target
- **Filters**: `isVisible`, `isEnabled` predicates on element lists
- **Matchers**: `stringContains`, `stringEquals`, `startsWith`, `endsWith`
- **waitFor predicates**: polling compose statements until they pass
- **Document actions**: `getUrl`, `getTitle`
- **Root pseudo-element**: `"element": "root"` references the page object's root

## Supported Actions

Every element supports these base actions:

| Action | Returns | Description |
|--------|---------|-------------|
| `getText` | String | Visible text content |
| `getAttribute` | String | Attribute value |
| `getClassAttribute` | String | CSS class names |
| `getValue` | String | Input value |
| `isVisible` | Bool | Whether displayed |
| `isEnabled` | Bool | Whether enabled |
| `isPresent` | Bool | Whether in DOM |
| `focus` | Null | Focus the element |
| `blur` | Null | Remove focus |
| `scrollIntoView` | Null | Scroll into view |
| `waitForVisible` | Null | Poll until visible |
| `waitForEnabled` | Null | Poll until enabled |

Clickable elements add: `click`, `doubleClick`, `rightClick`, `clickAndHold`

Editable elements add: `clear`, `setText`, `clearAndType`, `press`

Draggable elements add: `dragAndDropByOffset`

## Error Handling

All runtime operations return `RuntimeResult<T>`:

```rust
match page.call_method("login", &args).await {
    Ok(result) => println!("Success: {result}"),
    Err(RuntimeError::MethodNotFound { page_object, method }) => {
        println!("No method '{method}' on '{page_object}'");
    }
    Err(RuntimeError::ElementNotDefined { element, .. }) => {
        println!("Element '{element}' not found in page object");
    }
    Err(e) => println!("Error: {e}"),
}
```

## Cross-Page-Object Calls

Attach a registry to enable `applyExternal` resolution:

```rust
use std::sync::Arc;

let registry = Arc::new(registry);
let page = DynamicPageObject::load(driver, ast).await?
    .with_registry(registry);
```
