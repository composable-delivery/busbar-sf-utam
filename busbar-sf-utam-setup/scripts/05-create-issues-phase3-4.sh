#!/bin/bash
# UTAM Rust Project - Phase 3 & 4 Issues (CLI, Tooling, Integration)
# Run: ./05-create-issues-phase3-4.sh

set -e

REPO="composable-delivery/busbar-sf-utam"

echo "📋 Creating Phase 3 & 4 issues for $REPO..."

# ========== PHASE 3: CLI & TOOLING ==========
MILESTONE3="v0.3.0 - CLI & Tooling"

# Issue: CLI structure
gh issue create --repo "$REPO" \
  --title "[CLI] Implement CLI with clap" \
  --milestone "$MILESTONE3" \
  --label "component/cli,type/feature,priority/critical,size/L,copilot/good-prompt,status/ready" \
  --body "## Summary
Create the command-line interface using clap with subcommands.

## Acceptance Criteria
- [ ] \`utam compile\` - compile .utam.json to Rust
- [ ] \`utam validate\` - validate without generating
- [ ] \`utam init\` - create config file
- [ ] \`utam lint\` - lint with SARIF output
- [ ] Global options: --config, --verbose, --quiet
- [ ] Colored output and progress indicators
- [ ] Exit codes for CI integration

## Implementation
\`\`\`rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = \"utam\")]
#[command(author, version, about)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = \"utam.config.json\")]
    pub config: PathBuf,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compile UTAM JSON files to Rust
    Compile {
        /// Input files or directories
        #[arg(required = true)]
        input: Vec<PathBuf>,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Watch for changes
        #[arg(short, long)]
        watch: bool,
    },

    /// Validate UTAM JSON files
    Validate {
        /// Files to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output format (text, json, sarif)
        #[arg(long, default_value = \"text\")]
        format: String,
    },

    /// Initialize configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Lint UTAM JSON files
    Lint {
        /// Files to lint
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output SARIF report
        #[arg(long)]
        sarif: Option<PathBuf>,
    },
}
\`\`\`

## Copilot Prompt
\`\`\`
Create CLI for UTAM using clap with derive: Cli struct with global --config, --verbose,
--quiet options. Commands enum with Compile (input, output, watch), Validate (files, format),
Init (force), Lint (files, sarif). Add colored output with console crate. Return proper
exit codes (0=success, 1=error, 2=validation failed).
\`\`\`"

echo "✅ Created: CLI structure"

# Issue: Config file
gh issue create --repo "$REPO" \
  --title "[CLI] Implement configuration file support" \
  --milestone "$MILESTONE3" \
  --label "component/cli,type/feature,priority/high,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Support utam.config.json for project-level configuration.

## Acceptance Criteria
- [ ] Parse utam.config.json
- [ ] Support input/output paths
- [ ] Support file patterns (globs)
- [ ] Environment variable expansion
- [ ] Config merging with CLI args

## Configuration Schema
\`\`\`json
{
  \"\$schema\": \"./utam.config.schema.json\",
  \"inputDirectory\": \"src/pageobjects\",
  \"outputDirectory\": \"target/generated\",
  \"include\": [\"**/*.utam.json\"],
  \"exclude\": [\"**/test/**\"],
  \"compilerOptions\": {
    \"generateWaitMethods\": true,
    \"asyncRuntime\": \"tokio\",
    \"errorHandling\": \"result\"
  },
  \"lint\": {
    \"rules\": {
      \"require-description\": \"warn\",
      \"no-unused-elements\": \"error\"
    }
  }
}
\`\`\`

## Implementation
\`\`\`rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = \"camelCase\")]
pub struct UtamConfig {
    pub input_directory: PathBuf,
    pub output_directory: PathBuf,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub compiler_options: CompilerOptions,
    #[serde(default)]
    pub lint: LintConfig,
}

impl UtamConfig {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn find_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for pattern in &self.include {
            let full_pattern = self.input_directory.join(pattern);
            for entry in glob::glob(full_pattern.to_str().unwrap()).unwrap() {
                if let Ok(path) = entry {
                    if !self.is_excluded(&path) {
                        files.push(path);
                    }
                }
            }
        }
        files
    }
}
\`\`\`

## Copilot Prompt
\`\`\`
Implement config file support for UTAM CLI: UtamConfig struct with serde camelCase rename,
load() from JSON file, find_files() using glob patterns with exclude filtering. Support
compilerOptions and lint configuration. Add config JSON schema generation.
\`\`\`"

echo "✅ Created: Config file support"

# Issue: Watch mode
gh issue create --repo "$REPO" \
  --title "[CLI] Implement watch mode for development" \
  --milestone "$MILESTONE3" \
  --label "component/cli,type/feature,priority/medium,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Watch file system for changes and recompile automatically.

## Acceptance Criteria
- [ ] Watch input directories for changes
- [ ] Debounce rapid changes
- [ ] Only recompile changed files
- [ ] Clear terminal and show status
- [ ] Handle errors gracefully (don't exit)

## Implementation
\`\`\`rust
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

pub async fn watch_and_compile(config: &UtamConfig) -> Result<(), CliError> {
    let (tx, rx) = channel();

    let mut watcher = watcher(tx, Duration::from_millis(500))?;
    watcher.watch(&config.input_directory, RecursiveMode::Recursive)?;

    println!(\"🔄 Watching for changes in {:?}...\", config.input_directory);
    println!(\"   Press Ctrl+C to stop\");

    loop {
        match rx.recv() {
            Ok(event) => {
                if let Some(path) = event.path() {
                    if path.extension().map_or(false, |e| e == \"json\") {
                        clear_screen();
                        println!(\"📝 Change detected: {:?}\", path);

                        match compile_file(path, config) {
                            Ok(_) => println!(\"✅ Compiled successfully\"),
                            Err(e) => eprintln!(\"❌ Error: {}\", e),
                        }
                    }
                }
            }
            Err(e) => eprintln!(\"Watch error: {}\", e),
        }
    }
}
\`\`\`

## Copilot Prompt
\`\`\`
Implement watch mode for UTAM CLI using notify crate: watch input directory recursively,
debounce changes at 500ms, filter for .utam.json files, clear screen and recompile on
change. Handle errors without exiting. Show status messages with emoji indicators.
\`\`\`"

echo "✅ Created: Watch mode"

# Issue: SARIF output
gh issue create --repo "$REPO" \
  --title "[CLI] Implement SARIF linting output" \
  --milestone "$MILESTONE3" \
  --label "component/cli,type/feature,priority/medium,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Generate SARIF reports for integration with GitHub Code Scanning.

## Acceptance Criteria
- [ ] Valid SARIF 2.1.0 output
- [ ] Include file locations
- [ ] Map lint rules to SARIF rules
- [ ] Support severity levels
- [ ] Output to file or stdout

## Implementation
\`\`\`rust
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = \"camelCase\")]
pub struct SarifReport {
    #[serde(rename = \"\$schema\")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
#[serde(rename_all = \"camelCase\")]
pub struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
#[serde(rename_all = \"camelCase\")]
pub struct SarifResult {
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

impl SarifReport {
    pub fn new() -> Self {
        Self {
            schema: \"https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json\".to_string(),
            version: \"2.1.0\".to_string(),
            runs: vec![],
        }
    }

    pub fn add_lint_result(&mut self, lint: &LintResult) {
        // Convert lint result to SARIF format
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
\`\`\`

## Copilot Prompt
\`\`\`
Implement SARIF output for UTAM linter: SarifReport struct with version 2.1.0 schema,
SarifRun with tool info, SarifResult with rule_id, level, message, locations. Map lint
rules to SARIF rules. Include proper file URIs and region information for GitHub Code Scanning.
\`\`\`"

echo "✅ Created: SARIF output"

# ========== PHASE 4: INTEGRATION ==========
MILESTONE4="v0.4.0 - Integration"

# Issue: Test harness
gh issue create --repo "$REPO" \
  --title "[Integration] Create test harness utilities" \
  --milestone "$MILESTONE4" \
  --label "component/integration,type/feature,priority/high,size/L,copilot/good-prompt,status/ready" \
  --body "## Summary
Provide utilities for testing generated page objects.

## Acceptance Criteria
- [ ] WebDriver setup helpers (Chrome, Firefox)
- [ ] Test fixture management
- [ ] Screenshot on failure
- [ ] Retry mechanisms
- [ ] Parallel test support

## Implementation
\`\`\`rust
pub struct TestHarness {
    driver: WebDriver,
    screenshots_dir: PathBuf,
}

impl TestHarness {
    pub async fn new(browser: Browser) -> Result<Self, TestError> {
        let caps = match browser {
            Browser::Chrome => DesiredCapabilities::chrome(),
            Browser::Firefox => DesiredCapabilities::firefox(),
        };

        let driver = WebDriver::new(\"http://localhost:4444\", caps).await?;

        Ok(Self {
            driver,
            screenshots_dir: PathBuf::from(\"test-screenshots\"),
        })
    }

    pub async fn screenshot_on_failure<F, T>(&self, name: &str, f: F) -> Result<T, TestError>
    where
        F: std::future::Future<Output = Result<T, TestError>>,
    {
        match f.await {
            Ok(v) => Ok(v),
            Err(e) => {
                let path = self.screenshots_dir.join(format!(\"{}.png\", name));
                self.driver.screenshot(&path).await?;
                Err(e)
            }
        }
    }
}

#[macro_export]
macro_rules! utam_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() {
            let harness = TestHarness::new(Browser::Chrome).await.unwrap();
            harness.screenshot_on_failure(stringify!($name), async {
                $body
            }).await.unwrap();
        }
    };
}
\`\`\`

## Copilot Prompt
\`\`\`
Create test harness for UTAM: TestHarness struct with WebDriver setup for Chrome/Firefox,
screenshot_on_failure wrapper, retry mechanism with configurable attempts. Add utam_test!
macro for easy test creation. Support parallel test execution with separate driver instances.
\`\`\`"

echo "✅ Created: Test harness"

# Issue: Assertion helpers
gh issue create --repo "$REPO" \
  --title "[Integration] Implement assertion helpers" \
  --milestone "$MILESTONE4" \
  --label "component/integration,type/feature,priority/medium,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Provide assertion helpers for common page object testing patterns.

## Acceptance Criteria
- [ ] Element visibility assertions
- [ ] Text content assertions
- [ ] Attribute assertions
- [ ] Collection assertions
- [ ] Async-aware assertions with timeouts

## Implementation
\`\`\`rust
pub trait PageObjectAssertions {
    async fn assert_visible(&self) -> UtamResult<()>;
    async fn assert_hidden(&self) -> UtamResult<()>;
    async fn assert_text_equals(&self, expected: &str) -> UtamResult<()>;
    async fn assert_text_contains(&self, substring: &str) -> UtamResult<()>;
    async fn assert_attribute_equals(&self, name: &str, expected: &str) -> UtamResult<()>;
}

impl<T: ElementActions> PageObjectAssertions for T {
    async fn assert_visible(&self) -> UtamResult<()> {
        let visible = self.is_visible().await?;
        if !visible {
            return Err(UtamError::AssertionFailed {
                expected: \"element to be visible\".to_string(),
                actual: \"element is hidden\".to_string(),
            });
        }
        Ok(())
    }

    async fn assert_text_equals(&self, expected: &str) -> UtamResult<()> {
        let actual = self.get_text().await?;
        if actual != expected {
            return Err(UtamError::AssertionFailed {
                expected: format!(\"text '{}'\", expected),
                actual: format!(\"text '{}'\", actual),
            });
        }
        Ok(())
    }
}

/// Fluent assertion builder
pub struct ElementAssertion<'a, T> {
    element: &'a T,
    timeout: Duration,
}

impl<'a, T: ElementActions> ElementAssertion<'a, T> {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn is_visible(self) -> UtamResult<()> {
        wait_for(
            || async { Ok(self.element.is_visible().await.ok().filter(|v| *v)) },
            &WaitConfig { timeout: self.timeout, ..Default::default() },
            \"element visibility\",
        ).await?;
        Ok(())
    }
}
\`\`\`

## Copilot Prompt
\`\`\`
Implement assertion helpers for UTAM: PageObjectAssertions trait with assert_visible,
assert_hidden, assert_text_equals, assert_text_contains, assert_attribute_equals.
Add fluent ElementAssertion builder with timeout support. Return descriptive errors
with expected vs actual values.
\`\`\`"

echo "✅ Created: Assertion helpers"

# Issue: Salesforce compatibility
gh issue create --repo "$REPO" \
  --title "[Integration] Verify Salesforce page objects compatibility" \
  --milestone "$MILESTONE4" \
  --label "component/integration,type/feature,priority/high,size/L,copilot/good-prompt,status/needs-design" \
  --body "## Summary
Ensure compatibility with Salesforce Lightning page objects.

## Acceptance Criteria
- [ ] Parse salesforce-pageobjects Maven artifact
- [ ] Compile Salesforce page objects to Rust
- [ ] Handle Salesforce-specific patterns (Aura, LWC)
- [ ] Document any incompatibilities

## Tasks
1. Download salesforce-pageobjects source from Maven
2. Extract .utam.json files
3. Attempt compilation with utam-compiler
4. Document and fix any failures
5. Create compatibility test suite

## Known Salesforce Patterns
- Heavy use of shadow DOM
- Custom Aura components
- LWC components
- Dynamic component loading
- Slot-based composition

## Copilot Prompt
\`\`\`
Analyze Salesforce page objects for UTAM Rust compatibility: identify Salesforce-specific
patterns (Aura, LWC, shadow DOM), create test cases for each pattern, document any JSON
grammar features not yet implemented. Generate compatibility report.
\`\`\`"

echo "✅ Created: Salesforce compatibility"

# Issue: Example project
gh issue create --repo "$REPO" \
  --title "[Integration] Create example project with real-world usage" \
  --milestone "$MILESTONE4" \
  --label "component/integration,type/docs,priority/medium,size/L,copilot/good-prompt,status/ready" \
  --body "## Summary
Create a comprehensive example project demonstrating UTAM Rust usage.

## Acceptance Criteria
- [ ] Example page objects for TodoMVC app
- [ ] Complete test suite
- [ ] Documentation with walkthrough
- [ ] Docker setup for reproducible tests
- [ ] CI integration example

## Example Structure
\`\`\`
examples/
└── todomvc/
    ├── Cargo.toml
    ├── README.md
    ├── docker-compose.yml
    ├── utam.config.json
    ├── pageobjects/
    │   ├── app.utam.json
    │   ├── header.utam.json
    │   ├── todo-item.utam.json
    │   └── footer.utam.json
    ├── src/
    │   └── lib.rs
    └── tests/
        ├── add_todo.rs
        ├── complete_todo.rs
        └── filter_todos.rs
\`\`\`

## Example Page Object
\`\`\`json
{
  \"description\": \"TodoMVC application root\",
  \"root\": true,
  \"selector\": { \"css\": \".todoapp\" },
  \"elements\": [
    {
      \"name\": \"header\",
      \"type\": \"example/pageObjects/header\",
      \"selector\": { \"css\": \".header\" },
      \"public\": true
    },
    {
      \"name\": \"todoList\",
      \"selector\": { \"css\": \".todo-list\", \"returnAll\": true },
      \"type\": [\"clickable\"],
      \"public\": true
    }
  ],
  \"methods\": [
    {
      \"name\": \"addTodo\",
      \"compose\": [
        { \"element\": \"header\" },
        { \"chain\": true, \"apply\": \"enterTodo\", \"args\": [{ \"name\": \"text\", \"type\": \"string\" }] }
      ]
    }
  ]
}
\`\`\`

## Copilot Prompt
\`\`\`
Create TodoMVC example for UTAM Rust: page objects for app, header, todo-item, footer.
Include compose methods for addTodo, completeTodo, filterTodos. Write integration tests
using TestHarness. Add Docker setup with Selenium and TodoMVC app.
\`\`\`"

echo "✅ Created: Example project"

# Issue: Documentation generation
gh issue create --repo "$REPO" \
  --title "[Integration] Implement documentation generation" \
  --milestone "$MILESTONE4" \
  --label "component/integration,type/docs,priority/medium,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Generate documentation from UTAM JSON and Rust doc comments.

## Acceptance Criteria
- [ ] Extract descriptions from JSON
- [ ] Generate Rust doc comments
- [ ] Create mdBook documentation
- [ ] API reference generation
- [ ] Element/method index

## Implementation
\`\`\`rust
pub struct DocGenerator {
    pages: Vec<PageObjectDoc>,
}

#[derive(Serialize)]
pub struct PageObjectDoc {
    name: String,
    description: String,
    elements: Vec<ElementDoc>,
    methods: Vec<MethodDoc>,
}

impl DocGenerator {
    pub fn generate_rustdoc(&self, ast: &PageObjectAst) -> TokenStream {
        let doc = match &ast.description {
            Some(DescriptionAst::Simple(s)) => s.clone(),
            Some(DescriptionAst::Detailed { text, .. }) => text.join(\"\\n\"),
            None => String::new(),
        };

        quote! {
            #[doc = #doc]
        }
    }

    pub fn generate_mdbook(&self) -> String {
        let mut book = String::new();
        book.push_str(\"# Page Objects\\n\\n\");

        for page in &self.pages {
            book.push_str(&format!(\"## {}\\n\\n\", page.name));
            book.push_str(&format!(\"{}\\n\\n\", page.description));
            // ... more content
        }

        book
    }
}
\`\`\`

## Copilot Prompt
\`\`\`
Implement documentation generation for UTAM: DocGenerator that extracts descriptions
from JSON, generates Rust doc comments with quote!, and creates mdBook documentation
with page object index, element reference, and method documentation.
\`\`\`"

echo "✅ Created: Documentation generation"

echo ""
echo "📋 Phase 3 & 4 issues created!"
echo "   Phase 3: https://github.com/$REPO/issues?milestone=v0.3.0+-+CLI+%26+Tooling"
echo "   Phase 4: https://github.com/$REPO/issues?milestone=v0.4.0+-+Integration"
