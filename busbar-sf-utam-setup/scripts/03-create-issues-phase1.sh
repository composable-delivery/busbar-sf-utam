#!/bin/bash
# UTAM Rust Project - Phase 1 Issues (Core Runtime)
# Run: ./03-create-issues-phase1.sh

set -e

REPO="composable-delivery/busbar-sf-utam"
MILESTONE="v0.1.0 - Core Runtime"

echo "📋 Creating Phase 1 issues for $REPO..."

# Issue: Error types
gh issue create --repo "$REPO" \
  --title "[Core] Define UtamError and UtamResult types" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/critical,size/S,copilot/good-prompt,status/ready" \
  --body "## Summary
Define the core error types for the UTAM runtime library.

## Acceptance Criteria
- [ ] \`UtamError\` enum using thiserror
- [ ] \`UtamResult<T>\` type alias
- [ ] Error variants for: element not found, timeout, WebDriver errors, selector parse errors
- [ ] Proper error context with element names and selectors
- [ ] \`From\` implementations for thirtyfour errors

## Implementation
\`\`\`rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UtamError {
    #[error(\"Element '{name}' not found with selector: {selector}\")]
    ElementNotFound { name: String, selector: String },

    #[error(\"Timeout waiting for condition: {condition}\")]
    Timeout { condition: String },

    #[error(\"WebDriver error: {0}\")]
    WebDriver(#[from] thirtyfour::error::WebDriverError),

    #[error(\"Shadow root not found for element: {element}\")]
    ShadowRootNotFound { element: String },

    #[error(\"Invalid selector: {selector}\")]
    InvalidSelector { selector: String },

    #[error(\"Frame not found: {name}\")]
    FrameNotFound { name: String },
}

pub type UtamResult<T> = Result<T, UtamError>;
\`\`\`

## Tests
- [ ] Error messages are human-readable
- [ ] WebDriver errors convert correctly
- [ ] Error context is preserved

## Copilot Prompt
\`\`\`
Create error types for a UTAM Rust runtime using thiserror: UtamError enum with variants
for element not found, timeout, WebDriver errors, shadow root not found, invalid selector,
and frame not found. Each variant should include context fields. Create UtamResult type alias.
\`\`\`"

echo "✅ Created: Error types"

# Issue: Base element traits
gh issue create --repo "$REPO" \
  --title "[Core] Implement BaseElement trait with common actions" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/critical,size/L,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement the base element trait that all element types share.

## Acceptance Criteria
- [ ] \`BaseElement\` struct wrapping \`thirtyfour::WebElement\`
- [ ] All base actions from UTAM spec:
  - \`contains_element(selector, expand_shadow)\` → bool
  - \`get_attribute(name)\` → String
  - \`get_class_attribute()\` → String
  - \`get_css_property_value(name)\` → String
  - \`get_rect()\` → ElementRectangle
  - \`get_text()\` → String
  - \`get_title()\` → String
  - \`get_value()\` → String
  - \`is_enabled()\` → bool
  - \`is_focused()\` → bool
  - \`is_present()\` → bool
  - \`is_visible()\` → bool
- [ ] Async implementation using async-trait
- [ ] Proper error handling

## Implementation
\`\`\`rust
use async_trait::async_trait;
use thirtyfour::prelude::*;

pub struct BaseElement {
    inner: WebElement,
}

impl BaseElement {
    pub fn new(element: WebElement) -> Self {
        Self { inner: element }
    }

    pub fn inner(&self) -> &WebElement {
        &self.inner
    }
}

#[async_trait]
impl ElementActions for BaseElement {
    async fn get_text(&self) -> UtamResult<String> {
        Ok(self.inner.text().await?)
    }

    async fn get_attribute(&self, name: &str) -> UtamResult<Option<String>> {
        Ok(self.inner.attr(name).await?)
    }

    async fn is_visible(&self) -> UtamResult<bool> {
        Ok(self.inner.is_displayed().await?)
    }

    // ... more implementations
}
\`\`\`

## Tests
- [ ] All methods work on a test element
- [ ] Error cases handled (element stale, not found)
- [ ] Async operations complete correctly

## Copilot Prompt
\`\`\`
Implement BaseElement struct for UTAM wrapping thirtyfour::WebElement. Add async methods:
get_text, get_attribute, get_class_attribute, get_css_property_value, get_rect, get_title,
get_value, is_enabled, is_focused, is_present, is_visible, contains_element. Use async-trait.
Return UtamResult for all methods.
\`\`\`"

echo "✅ Created: Base element traits"

# Issue: Actionable trait
gh issue create --repo "$REPO" \
  --title "[Core] Implement Actionable trait (focus, blur, scroll)" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/high,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement the Actionable trait for basic user interaction actions.

## Acceptance Criteria
- [ ] \`Actionable\` trait definition
- [ ] \`ActionableElement\` struct implementing the trait
- [ ] Methods:
  - \`blur()\` - remove focus using JavaScript
  - \`focus()\` - set focus using JavaScript
  - \`move_to()\` - mouse hover using Actions API
  - \`scroll_to_center()\` - scroll element to viewport center
  - \`scroll_to_top()\` - scroll element to viewport top
- [ ] Proper JavaScript execution for blur/focus

## Implementation
\`\`\`rust
#[async_trait]
pub trait Actionable: Send + Sync {
    fn inner(&self) -> &WebElement;

    async fn blur(&self) -> UtamResult<()> {
        self.inner()
            .execute_script(\"arguments[0].blur()\")
            .await?;
        Ok(())
    }

    async fn focus(&self) -> UtamResult<()> {
        self.inner()
            .execute_script(\"arguments[0].focus()\")
            .await?;
        Ok(())
    }

    async fn move_to(&self) -> UtamResult<()> {
        self.inner().move_to().await?;
        Ok(())
    }

    async fn scroll_to_center(&self) -> UtamResult<()> {
        self.inner()
            .execute_script(
                \"arguments[0].scrollIntoView({block: 'center', inline: 'center'})\"
            )
            .await?;
        Ok(())
    }

    async fn scroll_to_top(&self) -> UtamResult<()> {
        self.inner()
            .execute_script(
                \"arguments[0].scrollIntoView({block: 'start', inline: 'start'})\"
            )
            .await?;
        Ok(())
    }
}

pub struct ActionableElement {
    base: BaseElement,
}
\`\`\`

## Tests
- [ ] Focus/blur change document.activeElement
- [ ] Scroll methods move viewport correctly
- [ ] move_to triggers hover state

## Copilot Prompt
\`\`\`
Implement Actionable trait for UTAM with async methods: blur, focus, move_to,
scroll_to_center, scroll_to_top. Use JavaScript execution for blur/focus, thirtyfour
Actions API for move_to, and scrollIntoView for scroll methods. Create ActionableElement
struct that wraps BaseElement and implements the trait.
\`\`\`"

echo "✅ Created: Actionable trait"

# Issue: Clickable trait
gh issue create --repo "$REPO" \
  --title "[Core] Implement Clickable trait (click, double-click, right-click)" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/high,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement the Clickable trait for click-related actions.

## Acceptance Criteria
- [ ] \`Clickable\` trait extending \`Actionable\`
- [ ] \`ClickableElement\` struct implementing the trait
- [ ] Methods:
  - \`click()\` - standard left click
  - \`double_click()\` - double click
  - \`right_click()\` - context menu click
  - \`click_and_hold(duration)\` - hold click for duration
- [ ] Use thirtyfour Actions API for complex clicks

## Implementation
\`\`\`rust
#[async_trait]
pub trait Clickable: Actionable {
    async fn click(&self) -> UtamResult<()> {
        self.inner().click().await?;
        Ok(())
    }

    async fn double_click(&self) -> UtamResult<()> {
        let driver = self.inner().driver();
        driver
            .action_chain()
            .double_click_element(self.inner())
            .perform()
            .await?;
        Ok(())
    }

    async fn right_click(&self) -> UtamResult<()> {
        let driver = self.inner().driver();
        driver
            .action_chain()
            .context_click_element(self.inner())
            .perform()
            .await?;
        Ok(())
    }

    async fn click_and_hold(&self, duration: Duration) -> UtamResult<()> {
        let driver = self.inner().driver();
        driver
            .action_chain()
            .click_and_hold_element(self.inner())
            .perform()
            .await?;
        tokio::time::sleep(duration).await;
        driver.action_chain().release().perform().await?;
        Ok(())
    }
}
\`\`\`

## Tests
- [ ] Click triggers onclick handler
- [ ] Double-click triggers ondblclick
- [ ] Right-click opens context menu (or triggers oncontextmenu)
- [ ] Click-and-hold maintains state for duration

## Copilot Prompt
\`\`\`
Implement Clickable trait for UTAM extending Actionable with async methods: click,
double_click, right_click, click_and_hold(duration). Use thirtyfour WebElement.click()
for simple click and ActionChain for double/right/hold clicks. Create ClickableElement
struct implementing the trait.
\`\`\`"

echo "✅ Created: Clickable trait"

# Issue: Editable trait
gh issue create --repo "$REPO" \
  --title "[Core] Implement Editable trait (type, clear, keyboard)" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/high,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement the Editable trait for text input actions.

## Acceptance Criteria
- [ ] \`Editable\` trait extending \`Actionable\`
- [ ] \`EditableElement\` struct implementing the trait
- [ ] Methods:
  - \`clear()\` - clear input value
  - \`clear_and_type(text)\` - clear then type
  - \`press(key)\` - press keyboard key
  - \`set_text(text)\` - type text without clearing
- [ ] Support for special keys (Enter, Tab, etc.)

## Implementation
\`\`\`rust
pub enum Key {
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    // ... more keys
}

impl From<Key> for thirtyfour::Key {
    fn from(key: Key) -> Self {
        match key {
            Key::Enter => thirtyfour::Key::Enter,
            Key::Tab => thirtyfour::Key::Tab,
            // ... mappings
        }
    }
}

#[async_trait]
pub trait Editable: Actionable {
    async fn clear(&self) -> UtamResult<()> {
        self.inner().clear().await?;
        Ok(())
    }

    async fn set_text(&self, text: &str) -> UtamResult<()> {
        self.inner().send_keys(text).await?;
        Ok(())
    }

    async fn clear_and_type(&self, text: &str) -> UtamResult<()> {
        self.clear().await?;
        self.set_text(text).await?;
        Ok(())
    }

    async fn press(&self, key: Key) -> UtamResult<()> {
        self.inner().send_keys(key.into()).await?;
        Ok(())
    }
}
\`\`\`

## Tests
- [ ] clear() empties input value
- [ ] set_text() appends to existing value
- [ ] clear_and_type() replaces value
- [ ] press() sends correct key codes

## Copilot Prompt
\`\`\`
Implement Editable trait for UTAM extending Actionable with async methods: clear,
set_text, clear_and_type, press(key). Create Key enum mapping to thirtyfour::Key.
Create EditableElement struct. Use WebElement.clear() and send_keys() for implementation.
\`\`\`"

echo "✅ Created: Editable trait"

# Issue: Draggable trait
gh issue create --repo "$REPO" \
  --title "[Core] Implement Draggable trait (drag-and-drop)" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/medium,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement the Draggable trait for drag-and-drop actions.

## Acceptance Criteria
- [ ] \`Draggable\` trait extending \`Actionable\`
- [ ] \`DraggableElement\` struct implementing the trait
- [ ] Methods:
  - \`drag_and_drop(target)\` - drag to another element
  - \`drag_and_drop_by_offset(x, y)\` - drag by pixel offset
  - Optional duration parameter for slow drag simulation
- [ ] Use thirtyfour ActionChain for drag operations

## Implementation
\`\`\`rust
#[async_trait]
pub trait Draggable: Actionable {
    async fn drag_and_drop(&self, target: &WebElement) -> UtamResult<()> {
        let driver = self.inner().driver();
        driver
            .action_chain()
            .drag_and_drop_element(self.inner(), target)
            .perform()
            .await?;
        Ok(())
    }

    async fn drag_and_drop_with_duration(
        &self,
        target: &WebElement,
        duration: Duration,
    ) -> UtamResult<()> {
        let driver = self.inner().driver();
        // Click and hold
        driver
            .action_chain()
            .click_and_hold_element(self.inner())
            .perform()
            .await?;

        // Wait for specified duration
        tokio::time::sleep(duration).await;

        // Move to target and release
        driver
            .action_chain()
            .move_to_element(target)
            .release()
            .perform()
            .await?;
        Ok(())
    }

    async fn drag_and_drop_by_offset(&self, x: i32, y: i32) -> UtamResult<()> {
        let driver = self.inner().driver();
        driver
            .action_chain()
            .drag_and_drop_by_offset(self.inner(), x, y)
            .perform()
            .await?;
        Ok(())
    }
}
\`\`\`

## Tests
- [ ] Drag to element moves source to target
- [ ] Drag by offset moves by correct pixels
- [ ] Duration parameter slows drag operation

## Copilot Prompt
\`\`\`
Implement Draggable trait for UTAM extending Actionable with async methods:
drag_and_drop(target), drag_and_drop_by_offset(x, y), drag_and_drop_with_duration.
Use thirtyfour ActionChain for all drag operations. Create DraggableElement struct.
\`\`\`"

echo "✅ Created: Draggable trait"

# Issue: Wait utilities
gh issue create --repo "$REPO" \
  --title "[Core] Implement wait utilities and predicates" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/high,size/L,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement wait utilities for element conditions and page loading.

## Acceptance Criteria
- [ ] \`wait_for<T>(predicate, timeout)\` - generic wait with predicate
- [ ] \`wait_for_visible(timeout)\` - wait for element visibility
- [ ] \`wait_for_invisible(timeout)\` - wait for element to hide
- [ ] \`wait_for_absence(timeout)\` - wait for element removal from DOM
- [ ] \`wait_for_enabled(timeout)\` - wait for element to be enabled
- [ ] Configurable polling interval
- [ ] Timeout error with helpful message

## Implementation
\`\`\`rust
use std::time::Duration;
use tokio::time::{interval, timeout};

pub struct WaitConfig {
    pub timeout: Duration,
    pub poll_interval: Duration,
}

impl Default for WaitConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            poll_interval: Duration::from_millis(500),
        }
    }
}

pub async fn wait_for<T, F, Fut>(
    predicate: F,
    config: &WaitConfig,
    description: &str,
) -> UtamResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = UtamResult<Option<T>>>,
{
    let start = std::time::Instant::now();
    let mut interval = interval(config.poll_interval);

    loop {
        interval.tick().await;

        match predicate().await? {
            Some(value) => return Ok(value),
            None if start.elapsed() > config.timeout => {
                return Err(UtamError::Timeout {
                    condition: description.to_string(),
                });
            }
            None => continue,
        }
    }
}

impl BaseElement {
    pub async fn wait_for_visible(&self, timeout: Duration) -> UtamResult<()> {
        wait_for(
            || async {
                if self.is_visible().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig { timeout, ..Default::default() },
            \"element to become visible\",
        )
        .await
    }

    pub async fn wait_for_absence(&self, timeout: Duration) -> UtamResult<()> {
        wait_for(
            || async {
                if !self.is_present().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig { timeout, ..Default::default() },
            \"element to be removed from DOM\",
        )
        .await
    }
}
\`\`\`

## Tests
- [ ] wait_for_visible succeeds when element appears
- [ ] wait_for_visible times out with error
- [ ] wait_for_absence succeeds when element removed
- [ ] Polling interval is respected

## Copilot Prompt
\`\`\`
Implement wait utilities for UTAM: generic wait_for function with predicate and timeout,
plus specific methods wait_for_visible, wait_for_invisible, wait_for_absence, wait_for_enabled.
Use tokio interval for polling. Return UtamError::Timeout with condition description on failure.
\`\`\`"

echo "✅ Created: Wait utilities"

# Issue: Shadow DOM support
gh issue create --repo "$REPO" \
  --title "[Core] Implement Shadow DOM support" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/critical,size/L,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement Shadow DOM traversal and element access within shadow roots.

## Acceptance Criteria
- [ ] \`ShadowRoot\` wrapper type
- [ ] \`get_shadow_root()\` method on elements
- [ ] Element finding within shadow roots
- [ ] Nested shadow DOM support
- [ ] Error handling for missing shadow roots

## Implementation
\`\`\`rust
pub struct ShadowRoot {
    inner: thirtyfour::ShadowRoot,
}

impl ShadowRoot {
    pub async fn find(&self, by: By) -> UtamResult<WebElement> {
        self.inner
            .find(by.clone())
            .await
            .map_err(|_| UtamError::ElementNotFound {
                name: \"shadow element\".to_string(),
                selector: format!(\"{:?}\", by),
            })
    }

    pub async fn find_all(&self, by: By) -> UtamResult<Vec<WebElement>> {
        Ok(self.inner.find_all(by).await?)
    }
}

impl BaseElement {
    pub async fn get_shadow_root(&self) -> UtamResult<ShadowRoot> {
        let shadow = self
            .inner
            .get_shadow_root()
            .await
            .map_err(|_| UtamError::ShadowRootNotFound {
                element: \"unknown\".to_string(),
            })?;
        Ok(ShadowRoot { inner: shadow })
    }
}

// Traversal helper for nested shadows
pub async fn traverse_shadow_path(
    root: &WebElement,
    path: &[By],
) -> UtamResult<WebElement> {
    let mut current = root.clone();

    for (i, selector) in path.iter().enumerate() {
        let shadow = current.get_shadow_root().await.map_err(|_| {
            UtamError::ShadowRootNotFound {
                element: format!(\"path element {}\", i),
            }
        })?;

        current = shadow.find(selector.clone()).await?;
    }

    Ok(current)
}
\`\`\`

## Tests
- [ ] Can access shadow root of element
- [ ] Can find elements within shadow root
- [ ] Nested shadows work correctly
- [ ] Error when element has no shadow root

## Copilot Prompt
\`\`\`
Implement Shadow DOM support for UTAM: ShadowRoot wrapper with find/find_all methods,
get_shadow_root method on BaseElement, and traverse_shadow_path helper for nested shadows.
Handle errors for missing shadow roots with UtamError::ShadowRootNotFound.
\`\`\`"

echo "✅ Created: Shadow DOM support"

# Issue: Container element
gh issue create --repo "$REPO" \
  --title "[Core] Implement Container element for dynamic content" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/high,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement Container element type for slots and dynamic content injection.

## Acceptance Criteria
- [ ] \`Container<T>\` generic struct
- [ ] Load method that finds and wraps content as page object
- [ ] Support for slot selectors
- [ ] Default selector of \`:scope > *:first-child\`
- [ ] Optional custom selector override

## Implementation
\`\`\`rust
use std::marker::PhantomData;

pub struct Container<T: PageObject> {
    root: WebElement,
    selector: Option<By>,
    _phantom: PhantomData<T>,
}

impl<T: PageObject> Container<T> {
    pub fn new(root: WebElement, selector: Option<By>) -> Self {
        Self {
            root,
            selector,
            _phantom: PhantomData,
        }
    }

    /// Load the contained page object
    pub async fn load(&self) -> UtamResult<T>
    where
        T: RootPageObject,
    {
        let selector = self
            .selector
            .clone()
            .unwrap_or_else(|| By::Css(\":scope > *:first-child\"));

        let element = self.root.find(selector).await?;
        T::from_element(element).await
    }

    /// Load with a specific type (for polymorphic slots)
    pub async fn load_as<U: RootPageObject>(&self) -> UtamResult<U> {
        let selector = self
            .selector
            .clone()
            .unwrap_or_else(|| By::Css(\":scope > *:first-child\"));

        let element = self.root.find(selector).await?;
        U::from_element(element).await
    }
}
\`\`\`

## Tests
- [ ] Container loads default first child
- [ ] Custom selector overrides default
- [ ] Works with different page object types
- [ ] Error when slot is empty

## Copilot Prompt
\`\`\`
Implement Container<T> for UTAM to handle slot/dynamic content: generic struct with root
element and optional selector, load() method that finds content and constructs page object,
load_as<U>() for polymorphic slots. Default selector is :scope > *:first-child.
\`\`\`"

echo "✅ Created: Container element"

# Issue: Frame element
gh issue create --repo "$REPO" \
  --title "[Core] Implement Frame element and context switching" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/high,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Implement Frame element type for iframe context switching.

## Acceptance Criteria
- [ ] \`FrameElement\` struct wrapping iframe elements
- [ ] \`FrameContext\` for RAII-style context management
- [ ] Switch to frame context
- [ ] Auto-switch back when context drops
- [ ] Nested frame support

## Implementation
\`\`\`rust
pub struct FrameElement {
    inner: WebElement,
}

impl FrameElement {
    pub fn new(element: WebElement) -> Self {
        Self { inner: element }
    }

    /// Enter the frame context
    pub async fn enter(&self) -> UtamResult<FrameContext> {
        let driver = self.inner.driver();
        driver.enter_frame(self.inner.clone().into()).await?;
        Ok(FrameContext { driver: driver.clone() })
    }
}

/// RAII guard for frame context - switches back to parent on drop
pub struct FrameContext {
    driver: WebDriver,
}

impl FrameContext {
    /// Find element within frame
    pub async fn find(&self, by: By) -> UtamResult<WebElement> {
        Ok(self.driver.find(by).await?)
    }

    /// Explicitly exit frame (or let it auto-exit on drop)
    pub async fn exit(self) -> UtamResult<()> {
        self.driver.enter_parent_frame().await?;
        std::mem::forget(self); // Prevent double-exit
        Ok(())
    }
}

impl Drop for FrameContext {
    fn drop(&mut self) {
        // Note: Can't await in drop, so we spawn a task
        // In practice, prefer explicit exit()
        let driver = self.driver.clone();
        tokio::spawn(async move {
            let _ = driver.enter_parent_frame().await;
        });
    }
}

// Usage example:
// let frame = page.get_content_frame().await?;
// let ctx = frame.enter().await?;
// let btn = ctx.find(By::Css(\".btn\")).await?;
// btn.click().await?;
// ctx.exit().await?;
\`\`\`

## Tests
- [ ] Can switch into iframe context
- [ ] Find works within frame
- [ ] Context switches back on exit
- [ ] Nested frames work correctly

## Copilot Prompt
\`\`\`
Implement Frame support for UTAM: FrameElement wrapping iframe WebElement with enter()
method returning FrameContext. FrameContext provides find() for in-frame queries and
exit() to return to parent. Use RAII pattern to auto-switch back on drop. Handle nested frames.
\`\`\`"

echo "✅ Created: Frame element"

# Issue: PageObject traits
gh issue create --repo "$REPO" \
  --title "[Core] Implement PageObject and RootPageObject traits" \
  --milestone "$MILESTONE" \
  --label "component/core,type/feature,priority/critical,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Define the core PageObject traits that all generated page objects implement.

## Acceptance Criteria
- [ ] \`PageObject\` trait for all page objects
- [ ] \`RootPageObject\` trait for loadable page objects
- [ ] \`load(driver)\` method for root page objects
- [ ] \`wait_for_load(driver, timeout)\` with configurable timeout
- [ ] \`from_element(element)\` constructor
- [ ] Root element accessor

## Implementation
\`\`\`rust
/// Trait implemented by all page objects
pub trait PageObject: Sized + Send + Sync {
    /// Get the root element of this page object
    fn root(&self) -> &WebElement;

    /// Get the WebDriver instance
    fn driver(&self) -> &WebDriver {
        self.root().driver()
    }
}

/// Trait for page objects that can be loaded directly (root=true)
#[async_trait]
pub trait RootPageObject: PageObject {
    /// The CSS selector for the root element
    const ROOT_SELECTOR: &'static str;

    /// Load the page object from the current page
    async fn load(driver: &WebDriver) -> UtamResult<Self>;

    /// Load with timeout for beforeLoad conditions
    async fn wait_for_load(
        driver: &WebDriver,
        timeout: Duration,
    ) -> UtamResult<Self> {
        let config = WaitConfig {
            timeout,
            ..Default::default()
        };

        wait_for(
            || async {
                match Self::load(driver).await {
                    Ok(po) => Ok(Some(po)),
                    Err(_) => Ok(None),
                }
            },
            &config,
            &format!(\"page object with selector '{}' to load\", Self::ROOT_SELECTOR),
        )
        .await
    }

    /// Construct from an existing element
    async fn from_element(element: WebElement) -> UtamResult<Self>;
}

// Example generated implementation:
// impl PageObject for LoginForm {
//     fn root(&self) -> &WebElement { &self.root }
// }
//
// #[async_trait]
// impl RootPageObject for LoginForm {
//     const ROOT_SELECTOR: &'static str = \"login-form\";
//
//     async fn load(driver: &WebDriver) -> UtamResult<Self> {
//         let root = driver.find(By::Css(Self::ROOT_SELECTOR)).await?;
//         Self::from_element(root).await
//     }
//
//     async fn from_element(element: WebElement) -> UtamResult<Self> {
//         Ok(Self { root: element, driver: element.driver().clone() })
//     }
// }
\`\`\`

## Tests
- [ ] PageObject trait compiles for test struct
- [ ] RootPageObject load works
- [ ] wait_for_load times out correctly
- [ ] from_element constructs valid page object

## Copilot Prompt
\`\`\`
Define PageObject and RootPageObject traits for UTAM: PageObject has root() and driver()
methods. RootPageObject extends with ROOT_SELECTOR const, load(), wait_for_load(), and
from_element(). Use async-trait. Provide example of generated implementation.
\`\`\`"

echo "✅ Created: PageObject traits"

echo ""
echo "📋 Phase 1 issues created! View at:"
echo "   https://github.com/$REPO/issues?milestone=v0.1.0+-+Core+Runtime"
