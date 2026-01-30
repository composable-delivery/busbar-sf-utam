#!/bin/bash
# UTAM Rust Project - Phase 2 Issues (Compiler)
# Run: ./04-create-issues-phase2.sh

set -e

REPO="composable-delivery/busbar-sf-utam"
MILESTONE="v0.2.0 - Compiler"

echo "📋 Creating Phase 2 issues for $REPO..."

# Issue: JSON Schema validation
gh issue create --repo "$REPO" \
  --title "[Compiler] Implement JSON schema validation" \
  --milestone "$MILESTONE" \
  --label "component/compiler,type/feature,priority/critical,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Validate UTAM JSON files against the official JSON schema before parsing.

## Acceptance Criteria
- [ ] Fetch and bundle UTAM JSON schema
- [ ] Validate input JSON against schema
- [ ] Report schema validation errors with line numbers
- [ ] Cache compiled schema for performance
- [ ] Support schema version detection

## Implementation
\`\`\`rust
use jsonschema::{JSONSchema, Draft};
use serde_json::Value;

pub struct SchemaValidator {
    schema: JSONSchema,
}

impl SchemaValidator {
    pub fn new() -> Result<Self, CompilerError> {
        let schema_json: Value = serde_json::from_str(include_str!(\"schema/utam-page-object.json\"))?;
        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_json)
            .map_err(|e| CompilerError::SchemaCompilation(e.to_string()))?;
        Ok(Self { schema })
    }

    pub fn validate(&self, json: &Value) -> Result<(), Vec<ValidationError>> {
        let result = self.schema.validate(json);
        if let Err(errors) = result {
            let validation_errors: Vec<_> = errors
                .map(|e| ValidationError {
                    path: e.instance_path.to_string(),
                    message: e.to_string(),
                })
                .collect();
            return Err(validation_errors);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}
\`\`\`

## Schema Source
Download from: https://www.schemastore.org/utam-page-object.json

## Tests
- [ ] Valid UTAM JSON passes validation
- [ ] Invalid JSON reports specific errors
- [ ] Schema path points to correct location
- [ ] Performance acceptable for large files

## Copilot Prompt
\`\`\`
Implement JSON schema validation for UTAM compiler using jsonschema crate: SchemaValidator
struct with new() that compiles bundled schema and validate() that returns detailed errors.
Use include_str! for embedded schema. Return ValidationError with path and message.
\`\`\`"

echo "✅ Created: JSON schema validation"

# Issue: AST data structures
gh issue create --repo "$REPO" \
  --title "[Compiler] Define complete AST data structures" \
  --milestone "$MILESTONE" \
  --label "component/compiler,type/feature,priority/critical,size/XL,copilot/good-prompt,status/needs-design" \
  --body "## Summary
Define all AST types that represent the parsed UTAM JSON structure.

## Acceptance Criteria
- [ ] \`PageObjectAst\` - root structure
- [ ] \`ElementAst\` - all element types (basic, custom, container, frame)
- [ ] \`SelectorAst\` - all selector types (CSS, mobile)
- [ ] \`MethodAst\` - compose method definitions
- [ ] \`ComposeStatementAst\` - method body statements
- [ ] \`FilterAst\` - element filters
- [ ] \`MatcherAst\` - filter matchers
- [ ] \`DescriptionAst\` - string or object description
- [ ] All types derive Serialize, Deserialize, Debug, Clone

## Implementation
\`\`\`rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageObjectAst {
    #[serde(default)]
    pub description: Option<DescriptionAst>,
    #[serde(default)]
    pub root: bool,
    pub selector: Option<SelectorAst>,
    #[serde(rename = \"exposeRootElement\", default)]
    pub expose_root_element: bool,
    #[serde(rename = \"type\", default)]
    pub action_types: Vec<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub implements: Option<String>,
    #[serde(rename = \"interface\", default)]
    pub is_interface: bool,
    #[serde(default)]
    pub shadow: Option<ShadowAst>,
    #[serde(default)]
    pub elements: Vec<ElementAst>,
    #[serde(default)]
    pub methods: Vec<MethodAst>,
    #[serde(rename = \"beforeLoad\", default)]
    pub before_load: Vec<ComposeStatementAst>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DescriptionAst {
    Simple(String),
    Detailed {
        text: Vec<String>,
        author: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementAst {
    pub name: String,
    #[serde(rename = \"type\")]
    pub element_type: ElementTypeAst,
    pub selector: Option<SelectorAst>,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub nullable: bool,
    #[serde(rename = \"wait\", default)]
    pub generate_wait: bool,
    #[serde(default)]
    pub load: bool,
    #[serde(default)]
    pub shadow: Option<ShadowAst>,
    #[serde(default)]
    pub elements: Vec<ElementAst>,
    #[serde(default)]
    pub filter: Option<FilterAst>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ElementTypeAst {
    /// Basic action types: [\"clickable\", \"editable\"]
    ActionTypes(Vec<String>),
    /// Custom component: \"package/pageObjects/component\"
    CustomComponent(String),
    /// Container literal
    #[serde(rename = \"container\")]
    Container,
    /// Frame literal
    #[serde(rename = \"frame\")]
    Frame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorAst {
    #[serde(default)]
    pub css: Option<String>,
    #[serde(default)]
    pub accessid: Option<String>,
    #[serde(default)]
    pub classchain: Option<String>,
    #[serde(default)]
    pub uiautomator: Option<String>,
    #[serde(default)]
    pub args: Vec<SelectorArgAst>,
    #[serde(rename = \"returnAll\", default)]
    pub return_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorArgAst {
    pub name: String,
    #[serde(rename = \"type\")]
    pub arg_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodAst {
    pub name: String,
    #[serde(default)]
    pub args: Vec<MethodArgAst>,
    #[serde(default)]
    pub compose: Vec<ComposeStatementAst>,
    #[serde(rename = \"returnType\")]
    pub return_type: Option<String>,
    #[serde(rename = \"returnAll\", default)]
    pub return_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodArgAst {
    pub name: String,
    #[serde(rename = \"type\")]
    pub arg_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeStatementAst {
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub apply: Option<String>,
    #[serde(default)]
    pub args: Vec<ComposeArgAst>,
    #[serde(default)]
    pub chain: bool,
    #[serde(rename = \"returnType\")]
    pub return_type: Option<String>,
    #[serde(rename = \"returnAll\", default)]
    pub return_all: bool,
    #[serde(default)]
    pub matcher: Option<MatcherAst>,
    #[serde(rename = \"applyExternal\")]
    pub apply_external: Option<ApplyExternalAst>,
}

// ... more types
\`\`\`

## Tests
- [ ] All example JSONs deserialize correctly
- [ ] Round-trip serialize/deserialize preserves data
- [ ] Optional fields handle missing values
- [ ] Unknown fields are ignored (forward compatibility)

## Copilot Prompt
\`\`\`
Define complete AST types for UTAM JSON grammar: PageObjectAst (root), ElementAst with
ElementTypeAst enum (action types, custom component, container, frame), SelectorAst for
CSS and mobile selectors, MethodAst with ComposeStatementAst, FilterAst with MatcherAst.
Use serde rename for JSON field mapping. Handle all optional fields.
\`\`\`"

echo "✅ Created: AST data structures"

# Issue: Element parsing
gh issue create --repo "$REPO" \
  --title "[Compiler] Implement element parsing for all types" \
  --milestone "$MILESTONE" \
  --label "component/compiler,type/feature,priority/high,size/L,copilot/good-prompt,status/ready" \
  --body "## Summary
Parse and validate all element types from UTAM JSON.

## Acceptance Criteria
- [ ] Basic elements with action types
- [ ] Custom component references (resolve package/path)
- [ ] Container elements with default selector
- [ ] Frame elements (no returnAll allowed)
- [ ] Nested elements within shadow boundaries
- [ ] Element filter parsing
- [ ] Validation of element names (unique, valid Rust identifiers)

## Implementation
\`\`\`rust
impl ElementAst {
    pub fn element_kind(&self) -> ElementKind {
        match &self.element_type {
            ElementTypeAst::ActionTypes(types) => {
                if types.is_empty() {
                    ElementKind::Basic
                } else {
                    ElementKind::Typed(types.clone())
                }
            }
            ElementTypeAst::CustomComponent(path) => {
                ElementKind::Custom(CustomComponentRef::parse(path))
            }
            ElementTypeAst::Container => ElementKind::Container,
            ElementTypeAst::Frame => ElementKind::Frame,
        }
    }
}

#[derive(Debug)]
pub enum ElementKind {
    Basic,
    Typed(Vec<String>),
    Custom(CustomComponentRef),
    Container,
    Frame,
}

#[derive(Debug)]
pub struct CustomComponentRef {
    pub package: String,
    pub path: Vec<String>,
    pub name: String,
}

impl CustomComponentRef {
    pub fn parse(s: &str) -> Self {
        let parts: Vec<&str> = s.split('/').collect();
        // Format: package/pageObjects/path/name
        Self {
            package: parts[0].to_string(),
            path: parts[2..parts.len()-1].iter().map(|s| s.to_string()).collect(),
            name: parts.last().unwrap().to_string(),
        }
    }

    pub fn to_rust_type(&self) -> String {
        // Convert kebab-case to PascalCase
        self.name
            .split('-')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().chain(c).collect(),
                }
            })
            .collect()
    }
}
\`\`\`

## Validation Rules
- Element name must be valid Rust identifier
- Frame elements cannot have returnAll: true
- Container default selector is :scope > *:first-child
- Custom component paths must be properly formatted

## Tests
- [ ] Parse basic element with types
- [ ] Parse custom component reference
- [ ] Container gets default selector
- [ ] Frame validates no returnAll
- [ ] Nested elements parse correctly

## Copilot Prompt
\`\`\`
Implement element parsing for UTAM compiler: element_kind() method on ElementAst returning
ElementKind enum (Basic, Typed, Custom, Container, Frame). Parse CustomComponentRef from
\"package/pageObjects/path/name\" format. Add to_rust_type() for PascalCase conversion.
Validate element constraints (frame no returnAll, unique names).
\`\`\`"

echo "✅ Created: Element parsing"

# Issue: Selector parsing
gh issue create --repo "$REPO" \
  --title "[Compiler] Implement selector parsing with parameters" \
  --milestone "$MILESTONE" \
  --label "component/compiler,type/feature,priority/high,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Parse CSS and mobile selectors, including parameterized selectors.

## Acceptance Criteria
- [ ] CSS selector parsing
- [ ] Mobile selectors (accessid, classchain, uiautomator)
- [ ] Parameterized selectors with %s and %d
- [ ] Generate code for parameter substitution
- [ ] Validate parameter count matches placeholders
- [ ] returnAll handling

## Implementation
\`\`\`rust
impl SelectorAst {
    pub fn selector_type(&self) -> SelectorType {
        if let Some(css) = &self.css {
            SelectorType::Css(css.clone())
        } else if let Some(accessid) = &self.accessid {
            SelectorType::AccessibilityId(accessid.clone())
        } else if let Some(classchain) = &self.classchain {
            SelectorType::IosClassChain(classchain.clone())
        } else if let Some(uiautomator) = &self.uiautomator {
            SelectorType::AndroidUiAutomator(uiautomator.clone())
        } else {
            SelectorType::Unknown
        }
    }

    pub fn has_parameters(&self) -> bool {
        !self.args.is_empty()
    }

    pub fn count_placeholders(&self) -> usize {
        let selector = match self.selector_type() {
            SelectorType::Css(s) => s,
            _ => return 0,
        };

        let string_count = selector.matches(\"%s\").count();
        let int_count = selector.matches(\"%d\").count();
        string_count + int_count
    }

    pub fn validate(&self) -> Result<(), SelectorError> {
        if self.has_parameters() {
            let placeholder_count = self.count_placeholders();
            let arg_count = self.args.len();
            if placeholder_count != arg_count {
                return Err(SelectorError::ParameterMismatch {
                    expected: placeholder_count,
                    actual: arg_count,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum SelectorType {
    Css(String),
    AccessibilityId(String),
    IosClassChain(String),
    AndroidUiAutomator(String),
    Unknown,
}

// Code generation for parameterized selector
pub fn generate_selector_code(selector: &SelectorAst) -> TokenStream {
    if selector.has_parameters() {
        let template = selector.css.as_ref().unwrap();
        let args: Vec<_> = selector.args.iter().map(|a| {
            let name = format_ident!(\"{}\", a.name);
            quote! { #name }
        }).collect();

        // Replace %s and %d with {} for format!
        let format_str = template
            .replace(\"%s\", \"{}\")
            .replace(\"%d\", \"{}\");

        quote! {
            By::Css(&format!(#format_str, #(#args),*))
        }
    } else {
        let css = selector.css.as_ref().unwrap();
        quote! { By::Css(#css) }
    }
}
\`\`\`

## Tests
- [ ] Parse simple CSS selector
- [ ] Parse parameterized selector
- [ ] Validate parameter count
- [ ] Generate correct format! code
- [ ] Handle mobile selectors

## Copilot Prompt
\`\`\`
Implement selector parsing for UTAM compiler: selector_type() returning SelectorType enum,
has_parameters() check, count_placeholders() for %s/%d, validate() for parameter count.
Generate code using quote! that creates format! call for parameterized selectors.
\`\`\`"

echo "✅ Created: Selector parsing"

# Issue: Compose method parsing
gh issue create --repo "$REPO" \
  --title "[Compiler] Implement compose method and statement parsing" \
  --milestone "$MILESTONE" \
  --label "component/compiler,type/feature,priority/high,size/L,copilot/good-prompt,status/ready" \
  --body "## Summary
Parse compose methods and their statement chains.

## Acceptance Criteria
- [ ] Method signature parsing (name, args, return type)
- [ ] Compose statement parsing
- [ ] Statement chaining logic
- [ ] Element reference resolution
- [ ] Argument reference handling
- [ ] Matcher parsing and validation

## Implementation
\`\`\`rust
impl MethodAst {
    pub fn rust_signature(&self) -> MethodSignature {
        MethodSignature {
            name: to_snake_case(&self.name),
            args: self.args.iter().map(|a| {
                RustArg {
                    name: to_snake_case(&a.name),
                    rust_type: utam_type_to_rust(&a.arg_type),
                }
            }).collect(),
            return_type: self.return_type.as_ref()
                .map(|t| utam_type_to_rust(t))
                .unwrap_or_else(|| \"()\".to_string()),
            is_async: true,
        }
    }
}

pub fn utam_type_to_rust(utam_type: &str) -> String {
    match utam_type {
        \"string\" => \"String\".to_string(),
        \"boolean\" => \"bool\".to_string(),
        \"number\" => \"i64\".to_string(),
        \"locator\" => \"By\".to_string(),
        \"function\" => \"/* predicate */\".to_string(),
        t if t.contains('/') => {
            // Custom type reference
            CustomComponentRef::parse(t).to_rust_type()
        }
        t => t.to_string(),
    }
}

#[derive(Debug)]
pub struct CompiledStatement {
    pub kind: StatementKind,
    pub return_type: Option<String>,
}

#[derive(Debug)]
pub enum StatementKind {
    /// Get element: self.get_element_name().await?
    GetElement { name: String },
    /// Apply action: element.action(args).await?
    ApplyAction { action: String, args: Vec<CompiledArg> },
    /// Chain from previous: prev.action(args).await?
    ChainAction { action: String, args: Vec<CompiledArg> },
    /// Matcher assertion
    MatcherAssert { matcher: MatcherKind, value: CompiledArg },
}

pub fn compile_compose_statements(
    statements: &[ComposeStatementAst],
    method_args: &[MethodArgAst],
    elements: &[ElementAst],
) -> Result<Vec<CompiledStatement>, CompilerError> {
    let mut compiled = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        let kind = if stmt.chain && i > 0 {
            // Chain from previous result
            StatementKind::ChainAction {
                action: stmt.apply.clone().unwrap_or_default(),
                args: compile_args(&stmt.args, method_args)?,
            }
        } else if let Some(element) = &stmt.element {
            if stmt.apply.is_some() {
                StatementKind::ApplyAction {
                    action: stmt.apply.clone().unwrap(),
                    args: compile_args(&stmt.args, method_args)?,
                }
            } else {
                StatementKind::GetElement { name: element.clone() }
            }
        } else {
            return Err(CompilerError::InvalidStatement(i));
        };

        compiled.push(CompiledStatement {
            kind,
            return_type: stmt.return_type.clone(),
        });
    }

    Ok(compiled)
}
\`\`\`

## Tests
- [ ] Parse simple method
- [ ] Parse chained statements
- [ ] Resolve element references
- [ ] Handle argument references
- [ ] Validate matcher types

## Copilot Prompt
\`\`\`
Implement compose method parsing for UTAM compiler: MethodAst.rust_signature() returning
typed signature, utam_type_to_rust() mapping, compile_compose_statements() that resolves
element/action references and handles chaining. StatementKind enum for GetElement,
ApplyAction, ChainAction. Handle argumentReference type.
\`\`\`"

echo "✅ Created: Compose method parsing"

# Issue: Code generation
gh issue create --repo "$REPO" \
  --title "[Compiler] Implement Rust code generation with quote" \
  --milestone "$MILESTONE" \
  --label "component/compiler,type/feature,priority/critical,size/XL,copilot/good-prompt,status/needs-design" \
  --body "## Summary
Generate Rust source code from parsed AST using the quote crate.

## Acceptance Criteria
- [ ] Generate struct definition for page object
- [ ] Generate PageObject trait implementation
- [ ] Generate RootPageObject trait implementation (if root=true)
- [ ] Generate private element getter methods
- [ ] Generate public element getter methods (if public=true)
- [ ] Generate wait methods (if wait=true)
- [ ] Generate compose methods
- [ ] Generate beforeLoad implementation
- [ ] Proper formatting with prettyplease

## Implementation
\`\`\`rust
use quote::{quote, format_ident};
use proc_macro2::TokenStream;

pub struct CodeGenerator {
    ast: PageObjectAst,
    config: CodeGenConfig,
}

impl CodeGenerator {
    pub fn generate(&self) -> String {
        let struct_def = self.generate_struct();
        let page_object_impl = self.generate_page_object_impl();
        let root_impl = self.generate_root_page_object_impl();
        let element_getters = self.generate_element_getters();
        let methods = self.generate_methods();

        let tokens = quote! {
            use utam_core::prelude::*;

            #struct_def

            #page_object_impl

            #root_impl

            impl #struct_name {
                #element_getters
                #methods
            }
        };

        // Format with prettyplease
        let syntax_tree = syn::parse2(tokens).unwrap();
        prettyplease::unparse(&syntax_tree)
    }

    fn generate_struct(&self) -> TokenStream {
        let name = format_ident!(\"{}\", self.struct_name());
        let doc = self.generate_doc_comment();

        quote! {
            #doc
            pub struct #name {
                root: WebElement,
                driver: WebDriver,
            }
        }
    }

    fn generate_element_getters(&self) -> TokenStream {
        let getters: Vec<_> = self.ast.all_elements().iter().map(|elem| {
            let method_name = format_ident!(\"get_{}\", to_snake_case(&elem.name));
            let visibility = if elem.public { quote! { pub } } else { quote! {} };
            let return_type = self.element_return_type(elem);
            let body = self.generate_element_body(elem);

            quote! {
                #visibility async fn #method_name(&self) -> UtamResult<#return_type> {
                    #body
                }
            }
        }).collect();

        quote! { #(#getters)* }
    }

    fn generate_compose_method(&self, method: &MethodAst) -> TokenStream {
        let name = format_ident!(\"{}\", to_snake_case(&method.name));
        let args = self.generate_method_args(method);
        let return_type = self.method_return_type(method);
        let body = self.generate_compose_body(&method.compose);

        quote! {
            pub async fn #name(&self, #args) -> UtamResult<#return_type> {
                #body
            }
        }
    }
}
\`\`\`

## Tests
- [ ] Generated code compiles
- [ ] Struct has correct fields
- [ ] Element getters work
- [ ] Methods have correct signatures
- [ ] beforeLoad is called in load()

## Copilot Prompt
\`\`\`
Implement Rust code generation for UTAM using quote crate: CodeGenerator struct with
generate() returning formatted Rust code. Generate struct definition, PageObject impl,
RootPageObject impl, element getters (private/public based on public field), compose
methods. Use prettyplease for formatting. Handle shadow DOM element paths.
\`\`\`"

echo "✅ Created: Code generation"

# Issue: Error reporting
gh issue create --repo "$REPO" \
  --title "[Compiler] Implement error reporting with source locations" \
  --milestone "$MILESTONE" \
  --label "component/compiler,type/feature,priority/high,size/M,copilot/good-prompt,status/ready" \
  --body "## Summary
Provide helpful error messages with file/line information.

## Acceptance Criteria
- [ ] Track source locations during parsing
- [ ] Include file path in errors
- [ ] Include line/column when possible
- [ ] Suggest fixes for common errors
- [ ] Colorized terminal output
- [ ] Machine-readable format option (JSON)

## Implementation
\`\`\`rust
use miette::{Diagnostic, NamedSource, SourceSpan};

#[derive(Debug, Diagnostic, Error)]
pub enum CompilerError {
    #[error(\"Invalid element type\")]
    #[diagnostic(
        code(utam::invalid_element_type),
        help(\"Element type must be an array of action types, a component path, 'container', or 'frame'\")
    )]
    InvalidElementType {
        #[source_code]
        src: NamedSource<String>,
        #[label(\"this element type is invalid\")]
        span: SourceSpan,
    },

    #[error(\"Unknown action type '{action}'\")]
    #[diagnostic(
        code(utam::unknown_action),
        help(\"Valid action types are: actionable, clickable, editable, draggable, touchable\")
    )]
    UnknownActionType {
        action: String,
        #[source_code]
        src: NamedSource<String>,
        #[label(\"unknown action type\")]
        span: SourceSpan,
    },

    #[error(\"Selector parameter mismatch: expected {expected}, found {actual}\")]
    #[diagnostic(
        code(utam::selector_params),
        help(\"Ensure the number of args matches the number of %s/%d placeholders in the selector\")
    )]
    SelectorParameterMismatch {
        expected: usize,
        actual: usize,
        #[source_code]
        src: NamedSource<String>,
        #[label(\"selector with {expected} placeholder(s)\")]
        span: SourceSpan,
    },
}

pub struct ErrorReporter {
    source: String,
    file_path: String,
}

impl ErrorReporter {
    pub fn report(&self, error: &CompilerError) {
        let report = miette::Report::new(error.clone());
        eprintln!(\"{:?}\", report);
    }

    pub fn report_json(&self, errors: &[CompilerError]) -> String {
        serde_json::to_string_pretty(&errors.iter().map(|e| {
            serde_json::json!({
                \"file\": self.file_path,
                \"message\": e.to_string(),
                \"code\": e.code().map(|c| c.to_string()),
            })
        }).collect::<Vec<_>>()).unwrap()
    }
}
\`\`\`

## Tests
- [ ] Errors include file path
- [ ] Errors include line number
- [ ] Help text is useful
- [ ] JSON format is valid
- [ ] Colors work in terminal

## Copilot Prompt
\`\`\`
Implement error reporting for UTAM compiler using miette: CompilerError enum with Diagnostic
derive for each variant, including source spans and help text. ErrorReporter struct with
report() for terminal output and report_json() for machine-readable format. Track source
locations during JSON parsing.
\`\`\`"

echo "✅ Created: Error reporting"

echo ""
echo "📋 Phase 2 issues created! View at:"
echo "   https://github.com/$REPO/issues?milestone=v0.2.0+-+Compiler"
