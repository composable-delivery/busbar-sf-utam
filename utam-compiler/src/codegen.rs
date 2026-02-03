//! Code generation module for UTAM compiler
//!
//! This module handles transformation of AST types into Rust source code.

use crate::ast::{ComposeArgAst, ComposeStatementAst, ElementAst, MethodArgAst, MethodAst};
use crate::error::{CompilerError, CompilerResult};

/// Rust method signature
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSignature {
    pub name: String,
    pub args: Vec<RustArg>,
    pub return_type: String,
    pub is_async: bool,
}

/// Rust method argument
#[derive(Debug, Clone, PartialEq)]
pub struct RustArg {
    pub name: String,
    pub rust_type: String,
}

/// Compiled argument that can be a literal value or a reference to a method argument
#[derive(Debug, Clone, PartialEq)]
pub enum CompiledArg {
    /// Literal value (e.g., "hello", 42, true)
    Literal(String),
    /// Reference to a method argument
    ArgumentReference(String),
}

/// Compiled statement ready for code generation
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledStatement {
    pub kind: StatementKind,
    pub return_type: Option<String>,
}

/// Kind of statement in a compose method
#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    /// Get element: self.get_element_name().await?
    GetElement { name: String },
    /// Apply action: element.action(args).await?
    ApplyAction {
        action: String,
        args: Vec<CompiledArg>,
    },
    /// Chain from previous: prev.action(args).await?
    ChainAction {
        action: String,
        args: Vec<CompiledArg>,
    },
    /// Matcher assertion
    MatcherAssert {
        matcher: MatcherKind,
        value: CompiledArg,
    },
}

/// Matcher types for element filtering
#[derive(Debug, Clone, PartialEq)]
pub enum MatcherKind {
    Contains,
    Equals,
    StartsWith,
    EndsWith,
}

impl MethodAst {
    /// Generate a Rust method signature from the UTAM method definition
    pub fn rust_signature(&self) -> MethodSignature {
        MethodSignature {
            name: to_snake_case(&self.name),
            args: self
                .args
                .iter()
                .map(|a| RustArg {
                    name: to_snake_case(&a.name),
                    rust_type: utam_type_to_rust(&a.arg_type),
                })
                .collect(),
            return_type: self
                .return_type
                .as_ref()
                .map(|t| utam_type_to_rust(t))
                .unwrap_or_else(|| "()".to_string()),
            is_async: true,
        }
    }
}

/// Convert UTAM type string to Rust type string
pub fn utam_type_to_rust(utam_type: &str) -> String {
    match utam_type {
        "string" => "String".to_string(),
        "boolean" => "bool".to_string(),
        "number" => "i64".to_string(),
        "locator" => "By".to_string(),
        "function" => "/* predicate */".to_string(),
        t if t.contains('/') => {
            // Custom type reference - extract the last component
            let parts: Vec<&str> = t.split('/').collect();
            if let Some(last) = parts.last() {
                // Convert to PascalCase for Rust type
                to_pascal_case(last)
            } else {
                t.to_string()
            }
        }
        t => t.to_string(),
    }
}

/// Compile compose statements into executable code structure
pub fn compile_compose_statements(
    statements: &[ComposeStatementAst],
    method_args: &[MethodArgAst],
    _elements: &[ElementAst],
) -> CompilerResult<Vec<CompiledStatement>> {
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
                StatementKind::GetElement {
                    name: element.clone(),
                }
            }
        } else if let Some(matcher) = &stmt.matcher {
            // Matcher assertion
            let matcher_kind = match matcher.matcher_type.as_str() {
                "contains" => MatcherKind::Contains,
                "equals" => MatcherKind::Equals,
                "startsWith" => MatcherKind::StartsWith,
                "endsWith" => MatcherKind::EndsWith,
                _ => {
                    return Err(CompilerError::InvalidStatement(format!(
                        "Unknown matcher type: {}",
                        matcher.matcher_type
                    )))
                }
            };
            let value = if let Some(first_arg) = matcher.args.first() {
                compile_single_arg(first_arg, method_args)?
            } else {
                return Err(CompilerError::InvalidStatement(
                    "Matcher requires an argument".to_string(),
                ));
            };
            StatementKind::MatcherAssert {
                matcher: matcher_kind,
                value,
            }
        } else {
            return Err(CompilerError::InvalidStatement(format!(
                "Invalid statement at index {}",
                i
            )));
        };

        compiled.push(CompiledStatement {
            kind,
            return_type: stmt.return_type.clone(),
        });
    }

    Ok(compiled)
}

/// Compile compose arguments into typed argument references
///
/// # Arguments
///
/// * `args` - The compose arguments to compile
/// * `method_args` - The method arguments for reference validation
///
/// # Returns
///
/// A vector of compiled arguments
///
/// # Errors
///
/// Returns `InvalidStatement` if an argument reference is not found in method arguments
fn compile_args(
    args: &[ComposeArgAst],
    method_args: &[MethodArgAst],
) -> CompilerResult<Vec<CompiledArg>> {
    args.iter()
        .map(|arg| compile_single_arg(arg, method_args))
        .collect()
}

/// Compile a single ComposeArgAst into a CompiledArg, validating argument references
///
/// # Arguments
///
/// * `arg` - The compose argument to compile
/// * `method_args` - The method arguments for reference validation
///
/// # Returns
///
/// A compiled argument (either a literal value or an argument reference)
///
/// # Errors
///
/// Returns `InvalidStatement` if an argument reference is not found in method arguments
fn compile_single_arg(arg: &ComposeArgAst, method_args: &[MethodArgAst]) -> CompilerResult<CompiledArg> {
    match arg {
        ComposeArgAst::Named { name, arg_type } => {
            // Check if this is an argumentReference
            if arg_type == "argumentReference" {
                // Verify the argument exists in method args
                if method_args.iter().any(|a| a.name == *name) {
                    Ok(CompiledArg::ArgumentReference(name.clone()))
                } else {
                    Err(CompilerError::InvalidStatement(format!(
                        "Argument reference '{}' not found in method arguments",
                        name
                    )))
                }
            } else {
                // Regular named argument - treat as reference
                Ok(CompiledArg::ArgumentReference(name.clone()))
            }
        }
        ComposeArgAst::Value(v) => {
            // Literal value
            let literal = if v.is_string() {
                format!("\"{}\"", v.as_str().unwrap_or(""))
            } else if v.is_boolean() {
                v.as_bool().unwrap_or(false).to_string()
            } else if v.is_number() {
                v.to_string()
            } else {
                v.to_string()
            };
            Ok(CompiledArg::Literal(literal))
        }
    }
}

/// Convert a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert a string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
//! Generates Rust source code from parsed AST using the quote crate.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::ast::*;
use crate::error::{CompilerError, CompilerResult};
use crate::utils::{to_pascal_case, to_snake_case};//! Codegen module for generating Rust code from UTAM AST
//!
//! This module provides functions to generate Rust code from parsed UTAM page objects.

use crate::ast::SelectorAst;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates Rust code for a selector, handling parameterized selectors
///
/// For parameterized selectors (with args), generates a format! call that
/// substitutes %s and %d placeholders with the provided arguments.
///
/// # Examples
///
/// Simple selector:
/// ```text
/// By::Css("button.submit")
/// ```
///
/// Parameterized selector with %s:
/// ```text
/// By::Css(&format!("button[data-id='{}']", button_id))
/// ```
pub fn generate_selector_code(selector: &SelectorAst) -> TokenStream {
    if selector.has_parameters() {
        // Get the selector string - we only support CSS for now with parameters
        let template = match selector.css.as_ref() {
            Some(css) => css,
            None => {
                // For non-CSS selectors with parameters, we'll need to handle them later
                return quote! { compile_error!("Parameterized selectors only supported for CSS") };
            }
        };

        // Generate the argument list
        let args: Vec<_> = selector
            .args
            .iter()
            .map(|a| {
                let name = format_ident!("{}", a.name);
                quote! { #name }
            })
            .collect();

        // Replace %s and %d with {} for format!
        let format_str = template.replace("%s", "{}").replace("%d", "{}");

        quote! {
            thirtyfour::By::Css(&format!(#format_str, #(#args),*))
        }
    } else {
        // Simple selector without parameters
        if let Some(css) = &selector.css {
            quote! { thirtyfour::By::Css(#css) }
        } else if let Some(accessid) = &selector.accessid {
            quote! { thirtyfour::By::Id(#accessid) }
        } else if let Some(classchain) = &selector.classchain {
            quote! { thirtyfour::By::IosClassChain(#classchain) }
        } else if let Some(uiautomator) = &selector.uiautomator {
            quote! { thirtyfour::By::AndroidUiAutomator(#uiautomator) }
        } else {
            quote! { compile_error!("Selector must have at least one selector type") }
        }
    }


/// Configuration for code generation
#[derive(Debug, Clone)]
pub struct CodeGenConfig {
    /// Module name for the generated code
    pub module_name: Option<String>,
}

impl Default for CodeGenConfig {
    fn default() -> Self {
        Self { module_name: None }
    }
}

/// Main code generator
pub struct CodeGenerator {
    ast: PageObjectAst,
    config: CodeGenConfig,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new(ast: PageObjectAst, config: CodeGenConfig) -> Self {
        Self { ast, config }
    }

    /// Generate Rust source code from AST
    pub fn generate(&self) -> CompilerResult<String> {
        let struct_name = self.struct_name();
        let struct_name_ident = format_ident!("{}", struct_name);

        let struct_def = self.generate_struct(&struct_name_ident);
        let page_object_impl = self.generate_page_object_impl(&struct_name_ident);
        let root_impl = if self.ast.root {
            Some(self.generate_root_page_object_impl(&struct_name_ident))
        } else {
            None
        };
        let element_getters = self.generate_element_getters(&struct_name_ident);
        let methods = self.generate_methods(&struct_name_ident);

        let tokens = quote! {
            use utam_core::prelude::*;

            #struct_def

            #page_object_impl

            #root_impl

            impl #struct_name_ident {
                #element_getters
                #methods
            }
        };

        // Format with prettyplease
        let syntax_tree = syn::parse2(tokens)
            .map_err(|e| CompilerError::Compilation(format!("Failed to parse generated tokens: {}", e)))?;
        Ok(prettyplease::unparse(&syntax_tree))
    }

    /// Get the struct name from module name or default
    fn struct_name(&self) -> String {
        self.config
            .module_name
            .as_ref()
            .map(|n| to_pascal_case(n))
            .unwrap_or_else(|| "PageObject".to_string())
    }

    /// Generate struct definition
    fn generate_struct(&self, struct_name: &proc_macro2::Ident) -> TokenStream {
        let doc = self.generate_doc_comment();

        quote! {
            #doc
            pub struct #struct_name {
                root: WebElement,
            }
        }
    }

    /// Generate doc comment for struct
    fn generate_doc_comment(&self) -> TokenStream {
        match &self.ast.description {
            Some(DescriptionAst::Simple(text)) => {
                quote! { #[doc = #text] }
            }
            Some(DescriptionAst::Detailed { text, author, .. }) => {
                let doc_lines: Vec<_> = text.iter().map(|line| {
                    quote! { #[doc = #line] }
                }).collect();
                
                let author_doc = if let Some(auth) = author {
                    let author_line = format!("\nAuthor: {}", auth);
                    quote! { #[doc = #author_line] }
                } else {
                    quote! {}
                };

                quote! {
                    #(#doc_lines)*
                    #author_doc
                }
            }
            None => quote! { #[doc = "Generated page object"] },
        }
    }

    /// Generate PageObject trait implementation
    fn generate_page_object_impl(&self, struct_name: &proc_macro2::Ident) -> TokenStream {
        quote! {
            impl PageObject for #struct_name {
                fn root(&self) -> &WebElement {
                    &self.root
                }
            }
        }
    }

    /// Generate RootPageObject trait implementation
    fn generate_root_page_object_impl(&self, struct_name: &proc_macro2::Ident) -> TokenStream {
        let selector = self.ast.selector.as_ref()
            .and_then(|s| s.css.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("");

        let before_load_body = if !self.ast.before_load.is_empty() {
            self.generate_before_load_body()
        } else {
            quote! { Ok(()) }
        };

        quote! {
            #[async_trait::async_trait]
            impl RootPageObject for #struct_name {
                const ROOT_SELECTOR: &'static str = #selector;

                async fn load(driver: &WebDriver) -> UtamResult<Self> {
                    // Execute beforeLoad if defined
                    Self::before_load(driver).await?;

                    let root = driver.find(By::Css(Self::ROOT_SELECTOR)).await?;
                    Self::from_element(root).await
                }

                async fn from_element(element: WebElement) -> UtamResult<Self> {
                    Ok(Self { root: element })
                }
            }

            impl #struct_name {
                /// Execute beforeLoad conditions
                async fn before_load(driver: &WebDriver) -> UtamResult<()> {
                    #before_load_body
                }
            }
        }
    }

    /// Generate beforeLoad method body
    fn generate_before_load_body(&self) -> TokenStream {
        let statements: Vec<_> = self.ast.before_load.iter().map(|stmt| {
            self.generate_compose_statement(stmt, None)
        }).collect();

        quote! {
            #(#statements)*
            Ok(())
        }
    }

    /// Generate element getter methods
    fn generate_element_getters(&self, _struct_name: &proc_macro2::Ident) -> TokenStream {
        let mut getters = Vec::new();

        // Get all elements including shadow elements
        for element in self.all_elements() {
            getters.push(self.generate_element_getter(&element));

            // If wait is true, generate a wait method
            if element.generate_wait {
                getters.push(self.generate_wait_method(&element));
            }
        }

        quote! { #(#getters)* }
    }

    /// Get all elements including shadow elements
    fn all_elements(&self) -> Vec<&ElementAst> {
        let mut elements = Vec::new();
        
        // Add regular elements
        for elem in &self.ast.elements {
            elements.push(elem);
        }
        
        // Add shadow elements
        if let Some(shadow) = &self.ast.shadow {
            for elem in &shadow.elements {
                elements.push(elem);
            }
        }
        
        elements
    }

    /// Generate a single element getter
    fn generate_element_getter(&self, element: &ElementAst) -> TokenStream {
        let method_name = format_ident!("get_{}", to_snake_case(&element.name));
        let visibility = if element.public {
            quote! { pub }
        } else {
            quote! {}
        };

        let return_type = self.element_return_type(element);
        let body = self.generate_element_body(element);
        let doc = if let Some(desc) = &element.description {
            quote! { #[doc = #desc] }
        } else {
            let doc_text = format!("Get the {} element", element.name);
            quote! { #[doc = #doc_text] }
        };

        quote! {
            #doc
            #visibility async fn #method_name(&self) -> UtamResult<#return_type> {
                #body
            }
        }
    }

    /// Generate wait method for an element
    fn generate_wait_method(&self, element: &ElementAst) -> TokenStream {
        let method_name = format_ident!("wait_for_{}", to_snake_case(&element.name));
        let getter_name = format_ident!("get_{}", to_snake_case(&element.name));
        let visibility = if element.public {
            quote! { pub }
        } else {
            quote! {}
        };

        let doc = format!("Wait for the {} element to be available", element.name);

        quote! {
            #[doc = #doc]
            #visibility async fn #method_name(&self, timeout: std::time::Duration) -> UtamResult<()> {
                let config = WaitConfig { timeout, ..Default::default() };
                wait_for(
                    || async {
                        match self.#getter_name().await {
                            Ok(_) => Ok(Some(())),
                            Err(_) => Ok(None),
                        }
                    },
                    &config,
                    "element to be available",
                )
                .await
            }
        }
    }

    /// Determine element return type
    fn element_return_type(&self, element: &ElementAst) -> TokenStream {
        if element.list {
            let inner_type = self.element_single_type(element);
            quote! { Vec<#inner_type> }
        } else {
            self.element_single_type(element)
        }
    }

    /// Determine single element type
    fn element_single_type(&self, element: &ElementAst) -> TokenStream {
        match &element.element_type {
            Some(ElementTypeAst::ActionTypes(types)) => {
                // Determine which element wrapper to use based on action types
                if types.iter().any(|t| t == "draggable") {
                    quote! { DraggableElement }
                } else if types.iter().any(|t| t == "editable") {
                    quote! { EditableElement }
                } else if types.iter().any(|t| t == "clickable") {
                    quote! { ClickableElement }
                } else if types.iter().any(|t| t == "actionable") {
                    quote! { BaseElement }
                } else {
                    quote! { BaseElement }
                }
            }
            Some(ElementTypeAst::CustomComponent(path)) => {
                // Convert path like "package/pageObjects/component" to PascalCase
                let component_name = path.split('/').last().unwrap_or(path);
                let ident = format_ident!("{}", to_pascal_case(component_name));
                quote! { #ident }
            }
            Some(ElementTypeAst::Container) => {
                quote! { ContainerElement }
            }
            Some(ElementTypeAst::Frame) => {
                // Frames return WebElement directly
                quote! { WebElement }
            }
            None => {
                quote! { BaseElement }
            }
        }
    }

    /// Generate element getter body
    fn generate_element_body(&self, element: &ElementAst) -> TokenStream {
        let selector = element.selector.as_ref()
            .and_then(|s| s.css.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("");

        let is_shadow = self.is_shadow_element(element);
        
        if element.list {
            // List of elements
            let wrapper_code = self.generate_element_wrapper(element);
            
            if is_shadow {
                quote! {
                    let shadow = self.root.get_shadow_root().await?;
                    let elements = shadow.find_all(By::Css(#selector)).await?;
                    let mut result = Vec::new();
                    for elem in elements {
                        #wrapper_code
                        result.push(wrapped);
                    }
                    Ok(result)
                }
            } else {
                quote! {
                    let elements = self.root.find_all(By::Css(#selector)).await?;
                    let mut result = Vec::new();
                    for elem in elements {
                        #wrapper_code
                        result.push(wrapped);
                    }
                    Ok(result)
                }
            }
        } else {
            // Single element
            let wrapper_code = self.generate_element_wrapper(element);
            
            if is_shadow {
                quote! {
                    let shadow = self.root.get_shadow_root().await?;
                    let elem = shadow.find(By::Css(#selector)).await?;
                    #wrapper_code
                    Ok(wrapped)
                }
            } else {
                quote! {
                    let elem = self.root.find(By::Css(#selector)).await?;
                    #wrapper_code
                    Ok(wrapped)
                }
            }
        }
    }

    /// Check if element is in shadow DOM
    fn is_shadow_element(&self, element: &ElementAst) -> bool {
        if let Some(shadow) = &self.ast.shadow {
            shadow.elements.iter().any(|e| e.name == element.name)
        } else {
            false
        }
    }

    /// Generate element wrapper code
    fn generate_element_wrapper(&self, element: &ElementAst) -> TokenStream {
        match &element.element_type {
            Some(ElementTypeAst::ActionTypes(types)) => {
                // Determine which element wrapper to use
                if types.iter().any(|t| t == "draggable") {
                    quote! {
                        let wrapped = DraggableElement::new(elem);
                    }
                } else if types.iter().any(|t| t == "editable") {
                    quote! {
                        let wrapped = EditableElement::new(elem);
                    }
                } else if types.iter().any(|t| t == "clickable") {
                    quote! {
                        let wrapped = ClickableElement::new(elem);
                    }
                } else {
                    quote! {
                        let wrapped = BaseElement::new(elem);
                    }
                }
            }
            Some(ElementTypeAst::CustomComponent(_path)) => {
                // For custom components, call from_element
                let component_type = self.element_single_type(element);
                quote! {
                    let wrapped = #component_type::from_element(elem).await?;
                }
            }
            Some(ElementTypeAst::Container) => {
                quote! {
                    let wrapped = ContainerElement::new(elem);
                }
            }
            Some(ElementTypeAst::Frame) => {
                // Frames don't need wrapping
                quote! {
                    let wrapped = elem;
                }
            }
            None => {
                quote! {
                    let wrapped = BaseElement::new(elem);
                }
            }
        }
    }

    /// Generate compose methods
    fn generate_methods(&self, _struct_name: &proc_macro2::Ident) -> TokenStream {
        let methods: Vec<_> = self.ast.methods.iter()
            .map(|method| self.generate_compose_method(method))
            .collect();

        quote! { #(#methods)* }
    }

    /// Generate a compose method
    fn generate_compose_method(&self, method: &MethodAst) -> TokenStream {
        let method_name = format_ident!("{}", to_snake_case(&method.name));
        let args = self.generate_method_args(method);
        let return_type = self.method_return_type(method);
        let body = self.generate_compose_body(&method.compose);
        
        let doc = match &method.description {
            Some(DescriptionAst::Simple(text)) => quote! { #[doc = #text] },
            Some(DescriptionAst::Detailed { text, .. }) => {
                let doc_lines: Vec<_> = text.iter().map(|line| {
                    quote! { #[doc = #line] }
                }).collect();
                quote! { #(#doc_lines)* }
            }
            None => {
                let doc_text = format!("{} method", method.name);
                quote! { #[doc = #doc_text] }
            }
        };

        quote! {
            #doc
            pub async fn #method_name(&self, #args) -> UtamResult<#return_type> {
                #body
            }
        }
    }

    /// Generate method arguments
    fn generate_method_args(&self, method: &MethodAst) -> TokenStream {
        // First, add explicit method args if they exist
        let mut args: Vec<TokenStream> = method.args.iter().map(|arg| {
            let arg_name = format_ident!("{}", to_snake_case(&arg.name));
            let arg_type = self.rust_type_from_string(&arg.arg_type);
            quote! { #arg_name: #arg_type }
        }).collect();

        // Then collect unique args from compose statements
        let mut arg_names = std::collections::HashSet::new();
        for arg in &method.args {
            arg_names.insert(arg.name.clone());
        }

        for stmt in &method.compose {
            for arg in &stmt.args {
                if let ComposeArgAst::Named { name, arg_type } = arg {
                    if arg_names.insert(name.clone()) {
                        let arg_name = format_ident!("{}", to_snake_case(name));
                        let rust_type = self.rust_type_from_string(arg_type);
                        args.push(quote! { #arg_name: #rust_type });
                    }
                }
            }
        }

        quote! { #(#args),* }
    }

    /// Determine method return type
    fn method_return_type(&self, method: &MethodAst) -> TokenStream {
        if let Some(return_type) = &method.return_type {
            let rust_type = self.rust_type_from_string(return_type);
            if method.return_all {
                quote! { Vec<#rust_type> }
            } else {
                quote! { #rust_type }
            }
        } else {
            quote! { () }
        }
    }

    /// Convert UTAM type string to Rust type
    fn rust_type_from_string(&self, type_str: &str) -> TokenStream {
        match type_str {
            "string" => quote! { &str },
            "boolean" => quote! { bool },
            "number" => quote! { i64 },
            _ => {
                // Assume it's a custom type
                let ident = format_ident!("{}", to_pascal_case(type_str));
                quote! { #ident }
            }
        }
    }

    /// Generate compose method body
    fn generate_compose_body(&self, statements: &[ComposeStatementAst]) -> TokenStream {
        let stmts: Vec<_> = statements.iter().enumerate().map(|(i, stmt)| {
            let is_last = i == statements.len() - 1;
            let last_result = if is_last { Some("result") } else { None };
            self.generate_compose_statement(stmt, last_result)
        }).collect();

        if statements.is_empty() {
            quote! { Ok(()) }
        } else if statements.iter().any(|s| s.return_element) {
            // If any statement returns an element, return it
            quote! {
                #(#stmts)*
                Ok(result)
            }
        } else {
            quote! {
                #(#stmts)*
                Ok(())
            }
        }
    }

    /// Generate a single compose statement
    fn generate_compose_statement(&self, stmt: &ComposeStatementAst, result_var: Option<&str>) -> TokenStream {
        if let Some(element_name) = &stmt.element {
            let getter_name = format_ident!("get_{}", to_snake_case(element_name));
            
            if let Some(apply) = &stmt.apply {
                let method_name = format_ident!("{}", to_snake_case(apply));
                let args = self.generate_compose_args(&stmt.args);
                
                if stmt.return_element || result_var.is_some() {
                    let var_name = format_ident!("{}", result_var.unwrap_or("result"));
                    quote! {
                        let #var_name = self.#getter_name().await?;
                        #var_name.#method_name(#args).await?;
                    }
                } else {
                    quote! {
                        let element = self.#getter_name().await?;
                        element.#method_name(#args).await?;
                    }
                }
            } else {
                // Just get the element
                if stmt.return_element || result_var.is_some() {
                    let var_name = format_ident!("{}", result_var.unwrap_or("result"));
                    quote! {
                        let #var_name = self.#getter_name().await?;
                    }
                } else {
                    quote! {
                        let _element = self.#getter_name().await?;
                    }
                }
            }
        } else if let Some(apply_external) = &stmt.apply_external {
            // External method call
            let method_name = format_ident!("{}", to_snake_case(&apply_external.method));
            let args = self.generate_compose_args(&apply_external.args);
            
            quote! {
                #method_name(#args).await?;
            }
        } else if let Some(apply) = &stmt.apply {
            // Direct apply without element (like waitFor on root)
            let method_name = format_ident!("{}", to_snake_case(apply));
            let args = self.generate_compose_args(&stmt.args);
            
            quote! {
                self.root.#method_name(#args).await?;
            }
        } else {
            quote! {}
        }
    }

    /// Generate arguments for compose statement
    fn generate_compose_args(&self, args: &[ComposeArgAst]) -> TokenStream {
        let arg_tokens: Vec<_> = args.iter().map(|arg| {
            match arg {
                ComposeArgAst::Named { name, .. } => {
                    let ident = format_ident!("{}", to_snake_case(name));
                    quote! { #ident }
                }
                ComposeArgAst::Value(value) => {
                    // Convert JSON value to Rust literal
                    match value {
                        serde_json::Value::String(s) => quote! { #s },
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                quote! { #i }
                            } else if let Some(f) = n.as_f64() {
                                quote! { #f }
                            } else {
                                quote! { 0 }
                            }
                        }
                        serde_json::Value::Bool(b) => quote! { #b },
                        _ => quote! { () },
                    }
                }
            }
        }).collect();

        quote! { #(#arg_tokens),* }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("myMethod"), "my_method");
        assert_eq!(to_snake_case("MyMethod"), "my_method");
        assert_eq!(to_snake_case("getElementList"), "get_element_list");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-component"), "MyComponent");
        assert_eq!(to_pascal_case("my_component"), "MyComponent");
        assert_eq!(to_pascal_case("component"), "Component");
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn test_utam_type_to_rust_basic() {
        assert_eq!(utam_type_to_rust("string"), "String");
        assert_eq!(utam_type_to_rust("boolean"), "bool");
        assert_eq!(utam_type_to_rust("number"), "i64");
        assert_eq!(utam_type_to_rust("locator"), "By");
    }

    #[test]
    fn test_utam_type_to_rust_custom() {
        assert_eq!(
            utam_type_to_rust("utam-applications/pageObjects/component"),
            "Component"
        );
        assert_eq!(
            utam_type_to_rust("package/pageObjects/my-button"),
            "MyButton"
        );
    }

    #[test]
    fn test_method_rust_signature() {
        let method = MethodAst {
            name: "loginUser".to_string(),
            description: None,
            args: vec![
                MethodArgAst {
                    name: "username".to_string(),
                    arg_type: "string".to_string(),
                },
                MethodArgAst {
                    name: "password".to_string(),
                    arg_type: "string".to_string(),
                },
            ],
            compose: vec![],
            return_type: None,
            return_all: false,
        };

        let sig = method.rust_signature();
        assert_eq!(sig.name, "login_user");
        assert_eq!(sig.args.len(), 2);
        assert_eq!(sig.args[0].name, "username");
        assert_eq!(sig.args[0].rust_type, "String");
        assert_eq!(sig.return_type, "()");
        assert!(sig.is_async);
    }

    #[test]
    fn test_compile_single_arg_literal() {
        let arg = ComposeArgAst::Value(serde_json::json!("test"));
        let method_args = vec![];
        let compiled = compile_single_arg(&arg, &method_args).unwrap();
        assert_eq!(compiled, CompiledArg::Literal("\"test\"".to_string()));
    }

    #[test]
    fn test_compile_single_arg_reference() {
        let arg = ComposeArgAst::Named {
            name: "username".to_string(),
            arg_type: "argumentReference".to_string(),
        };
        let method_args = vec![MethodArgAst {
            name: "username".to_string(),
            arg_type: "string".to_string(),
        }];
        let compiled = compile_single_arg(&arg, &method_args).unwrap();
        assert_eq!(compiled, CompiledArg::ArgumentReference("username".to_string()));
    }

    #[test]
    fn test_compile_compose_statements_get_element() {
        let statements = vec![ComposeStatementAst {
            element: Some("submitButton".to_string()),
            apply: None,
            args: vec![],
            chain: false,
            return_type: None,
            return_all: false,
            matcher: None,
            apply_external: None,
            filter: None,
            return_element: false,
            predicate: None,
        }];

        let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
        assert_eq!(compiled.len(), 1);
        match &compiled[0].kind {
            StatementKind::GetElement { name } => {
                assert_eq!(name, "submitButton");
            }
            _ => panic!("Expected GetElement"),
        }
    }

    #[test]
    fn test_compile_compose_statements_apply_action() {
        let statements = vec![ComposeStatementAst {
            element: Some("usernameInput".to_string()),
            apply: Some("clearAndType".to_string()),
            args: vec![ComposeArgAst::Named {
                name: "username".to_string(),
                arg_type: "argumentReference".to_string(),
            }],
            chain: false,
            return_type: None,
            return_all: false,
            matcher: None,
            apply_external: None,
            filter: None,
            return_element: false,
            predicate: None,
        }];

        let method_args = vec![MethodArgAst {
            name: "username".to_string(),
            arg_type: "string".to_string(),
        }];

        let compiled = compile_compose_statements(&statements, &method_args, &[]).unwrap();
        assert_eq!(compiled.len(), 1);
        match &compiled[0].kind {
            StatementKind::ApplyAction { action, args } => {
                assert_eq!(action, "clearAndType");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected ApplyAction"),
        }
    use crate::ast::SelectorArgAst;

    #[test]
    fn test_generate_simple_css_selector() {
        let selector = SelectorAst {
            css: Some("button.submit".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        let code = generate_selector_code(&selector);
        let code_str = code.to_string();
        assert!(code_str.contains("thirtyfour :: By :: Css"));
        assert!(code_str.contains("button.submit"));
    }

    #[test]
    fn test_generate_parameterized_selector_with_string() {
        let selector = SelectorAst {
            css: Some("button[data-id='%s']".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst {
                name: "button_id".to_string(),
                arg_type: "string".to_string(),
            }],
            return_all: false,
        };

        let code = generate_selector_code(&selector);
        let code_str = code.to_string();
        // TokenStream adds spaces between tokens, so "format!" becomes "format !"
        assert!(code_str.contains("format !"));
        assert!(code_str.contains("button_id"));
        assert!(code_str.contains("{}"));
    }

    #[test]
    fn test_generate_parameterized_selector_with_number() {
        let selector = SelectorAst {
            css: Some("li:nth-child(%d)".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst {
                name: "index".to_string(),
                arg_type: "number".to_string(),
            }],
            return_all: false,
        };

        let code = generate_selector_code(&selector);
        let code_str = code.to_string();
        // TokenStream adds spaces between tokens
        assert!(code_str.contains("format !"));
        assert!(code_str.contains("index"));
    }

    #[test]
    fn test_generate_mobile_selector_accessid() {
        let selector = SelectorAst {
            css: None,
            accessid: Some("submit-button".to_string()),
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        let code = generate_selector_code(&selector);
        let code_str = code.to_string();
        assert!(code_str.contains("thirtyfour :: By :: Id"));
        assert!(code_str.contains("submit-button"));
    fn test_generate_simple_page_object() {
        let ast = PageObjectAst {
            description: Some(DescriptionAst::Simple("Test page".to_string())),
            root: true,
            selector: Some(SelectorAst {
                css: Some(".test".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: false,
            }),
            expose_root_element: false,
            action_types: vec![],
            platform: None,
            implements: None,
            is_interface: false,
            shadow: None,
            elements: vec![],
            methods: vec![],
            before_load: vec![],
            metadata: None,
        };

        let config = CodeGenConfig {
            module_name: Some("TestPage".to_string()),
        };

        let generator = CodeGenerator::new(ast, config);
        let code = generator.generate().unwrap();

        assert!(code.contains("pub struct TestPage"));
        assert!(code.contains("impl PageObject for TestPage"));
        assert!(code.contains("impl RootPageObject for TestPage"));
        assert!(code.contains("const ROOT_SELECTOR: &'static str = \".test\""));
    }

    #[test]
    fn test_generate_with_elements() {
        let ast = PageObjectAst {
            description: None,
            root: true,
            selector: Some(SelectorAst {
                css: Some(".form".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: false,
            }),
            expose_root_element: false,
            action_types: vec![],
            platform: None,
            implements: None,
            is_interface: false,
            shadow: None,
            elements: vec![ElementAst {
                name: "submitButton".to_string(),
                element_type: Some(ElementTypeAst::ActionTypes(vec!["clickable".to_string()])),
                selector: Some(SelectorAst {
                    css: Some("button[type='submit']".to_string()),
                    accessid: None,
                    classchain: None,
                    uiautomator: None,
                    args: vec![],
                    return_all: false,
                }),
                public: true,
                nullable: false,
                generate_wait: false,
                load: false,
                shadow: None,
                elements: vec![],
                filter: None,
                description: None,
                list: false,
            }],
            methods: vec![],
            before_load: vec![],
            metadata: None,
        };

        let config = CodeGenConfig {
            module_name: Some("TestForm".to_string()),
        };

        let generator = CodeGenerator::new(ast, config);
        let code = generator.generate().unwrap();

        assert!(code.contains("pub async fn get_submit_button"));
        assert!(code.contains("ClickableElement"));
    }

    #[test]
    fn test_generate_with_compose_method() {
        let ast = PageObjectAst {
            description: None,
            root: true,
            selector: Some(SelectorAst {
                css: Some(".login".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: false,
            }),
            expose_root_element: false,
            action_types: vec![],
            platform: None,
            implements: None,
            is_interface: false,
            shadow: None,
            elements: vec![
                ElementAst {
                    name: "usernameInput".to_string(),
                    element_type: Some(ElementTypeAst::ActionTypes(vec!["editable".to_string()])),
                    selector: Some(SelectorAst {
                        css: Some("input[name='username']".to_string()),
                        accessid: None,
                        classchain: None,
                        uiautomator: None,
                        args: vec![],
                        return_all: false,
                    }),
                    public: false,
                    nullable: false,
                    generate_wait: false,
                    load: false,
                    shadow: None,
                    elements: vec![],
                    filter: None,
                    description: None,
                    list: false,
                },
            ],
            methods: vec![MethodAst {
                name: "setUsername".to_string(),
                description: None,
                args: vec![],
                compose: vec![ComposeStatementAst {
                    element: Some("usernameInput".to_string()),
                    apply: Some("clearAndType".to_string()),
                    args: vec![ComposeArgAst::Named {
                        name: "username".to_string(),
                        arg_type: "string".to_string(),
                    }],
                    chain: false,
                    return_type: None,
                    return_all: false,
                    matcher: None,
                    apply_external: None,
                    filter: None,
                    return_element: false,
                    predicate: None,
                }],
                return_type: None,
                return_all: false,
            }],
            before_load: vec![],
            metadata: None,
        };

        let config = CodeGenConfig {
            module_name: Some("LoginForm".to_string()),
        };

        let generator = CodeGenerator::new(ast, config);
        let code = generator.generate().unwrap();

        assert!(code.contains("pub async fn set_username"));
        assert!(code.contains("username: &str"));
        assert!(code.contains("clear_and_type"));
    }
}
