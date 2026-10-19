//! Tungsten's dev Babel plugins, ported to [`oxc_ti_compile::Passes`].
//!
//! [`TungstenPasses`] runs these, in the order tungsten's Babel dev config lists them:
//!
//! 1. `jsxExtensionsToJs` - `BabelTransformJsxExtensionsToJs`
//! 2. `packageImports` - the AST half of `BabelTransformPackageImports`; resolving and compiling
//!    packages stays in JS
//! 3. `cssImports` - `BabelTransformCssImports`
//! 4. `componentStyles` - the AST half of `BabelTransformComponentStyles`; Sass, postcss, the cache,
//!    the watcher and the CSS writes stay in JS
//! 5. `docInit` - `BabelTransformDocInit`, without writing the head script
//! 6. `devCacheBust` - `BabelAddDevCacheBust`
//!
//! Babel merges plugin visitors into one traversal that runs ahead of the presets, so the passes
//! share one traversal in [`Passes::before`], each node's visitors in plugin order. Two things wait
//! for [`Passes::after`], when TypeScript and JSX are compiled: DocInit's call rewrite (its Babel
//! plugin rewrites at `Program.exit`, after the presets) and the `bareImports` list, which must leave
//! out imports TypeScript removes and include the JSX runtime import.
//!
//! A [`TungstenPasses`] is for one compile: make one per file.
//!
//! # Options JSON
//!
//! A pass runs only when its key is present.
//!
//! ```json
//! {
//!   "jsxExtensionsToJs": true,
//!   "packageImports": { "replacements": { "pkg/theme.css": "/.packages/pkg@1.2.3/theme.css" } },
//!   "cssImports": { "absCodeDir": "C:/app/code", "absSourceDir": "C:/app/src" },
//!   "componentStyles": {
//!     "results": [{
//!       "moduleName": "pages-Home",
//!       "tags": { "div": "pages-Home_div" },
//!       "classes": { "box": "pages-Home_box" },
//!       "keyframes": {},
//!       "stylesheet": "i/pages-Home.css"
//!     }]
//!   },
//!   "docInit": { "baseId": 0 },
//!   "devCacheBust": true
//! }
//! ```
//!
//! - `packageImports.replacements` (optional): bare specifier, as `bareImports` lists it, to the
//!   source it becomes. JS supplies one only for a target without a JavaScript extension: Babel
//!   rewrites only those, and records the rest as import map aliases.
//! - `componentStyles.results` (optional): one entry per style template, in `componentStyles` output
//!   order. Without it the templates are only listed (phase 1); with it each becomes its object and a
//!   template in a class adds `static $stylesheet = <stylesheet>` to it (phase 2).
//! - `docInit.baseId` (default 0): the id of the file's first `DocInit` call; later calls count up.
//!
//! # Output JSON
//!
//! [`TungstenPasses::output_json`] has a key per enabled pass that reports anything:
//!
//! ```json
//! {
//!   "bareImports": ["tungsten/jsx-runtime", "pkg/theme.css"],
//!   "componentStyles": [{ "index": 1, "tag": "Scss", "raw": ".box { color: red }", "inClass": true }],
//!   "docInit": [{ "id": 0, "function": "() => {\n\treturn 1;\n}", "line": 3, "column": 26 }]
//! }
//! ```
//!
//! - `bareImports`: the bare import specifiers Babel's visitor would see, original (unreplaced) and
//!   deduplicated, for JS to resolve.
//! - `componentStyles`: each `Css`/`Sass`/`Scss`/`ReactZoneScss` template in traversal order: its
//!   1-based index in the file (Babel's `stylesheetIndex`), tag, the raw text of its first quasi, and
//!   whether it is inside a class body.
//! - `docInit`: each rewritten call: its id, the function compiled (TypeScript removed, JSX compiled,
//!   no comments) for the head script, and where the function starts in the source, 1-based line and
//!   1-based UTF-16 column as Babel's call site reports them.
//!
//! Errors and warnings go to the compile's diagnostics. A warning is what Babel only logs.

mod component_styles;
mod css_imports;
mod dev_cache_bust;
mod doc_init;
mod js_path;
mod jsx_extensions_to_js;
mod package_imports;

use std::path::{Path, PathBuf};

use oxc_ast::ast::{CallExpression, ClassBody, Expression, Program, Statement};
use oxc_diagnostics::OxcDiagnostic;
use oxc_semantic::Scoping;
use oxc_ti_compile::{PassContext, Passes};
use oxc_traverse::{BoundIdentifier, Traverse, traverse_mut};
use serde::{Deserialize, Serialize};

use component_styles::{ComponentStyles, ComponentStylesOptions};
use css_imports::{CssImports, CssImportsOptions};
use doc_init::{DocInit, DocInitOptions};
use package_imports::{PackageImports, PackageImportsOptions};

type TraverseCtx<'a> = oxc_traverse::TraverseCtx<'a, ()>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Options {
    #[serde(default)]
    jsx_extensions_to_js: bool,
    package_imports: Option<PackageImportsOptions>,
    css_imports: Option<CssImportsOptions>,
    component_styles: Option<ComponentStylesOptions>,
    doc_init: Option<DocInitOptions>,
    #[serde(default)]
    dev_cache_bust: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Output<'p> {
    #[serde(skip_serializing_if = "Option::is_none")]
    bare_imports: Option<&'p [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    component_styles: Option<&'p [component_styles::ExtractedStyle]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc_init: Option<&'p [doc_init::Entry]>,
}

pub struct TungstenPasses {
    jsx_extensions_to_js: bool,
    package_imports: Option<PackageImports>,
    css_imports: Option<CssImports>,
    component_styles: Option<ComponentStyles>,
    doc_init: Option<DocInit>,
    dev_cache_bust: bool,
}

impl TungstenPasses {
    /// # Errors
    /// When the JSON does not have the shape the crate docs give.
    pub fn from_options_json(json: &str) -> Result<Self, String> {
        let options: Options = serde_json::from_str(json).map_err(|error| error.to_string())?;
        Ok(Self {
            jsx_extensions_to_js: options.jsx_extensions_to_js,
            package_imports: options.package_imports.map(PackageImports::new),
            css_imports: options.css_imports.map(CssImports::new),
            component_styles: options.component_styles.map(ComponentStyles::new),
            doc_init: options.doc_init.map(DocInit::new),
            dev_cache_bust: options.dev_cache_bust,
        })
    }

    /// What the passes found for JS, as the crate docs describe.
    ///
    /// # Panics
    /// Never: the output is strings and numbers, which always serialize.
    pub fn output_json(&self) -> String {
        let output = Output {
            bare_imports: self.package_imports.as_ref().map(PackageImports::bare_imports),
            component_styles: self.component_styles.as_ref().map(ComponentStyles::extracted),
            doc_init: self.doc_init.as_ref().map(DocInit::entries),
        };
        serde_json::to_string(&output).expect("output serializes")
    }
}

impl Passes for TungstenPasses {
    fn before(&mut self, context: PassContext<'_, '_>, scoping: Scoping) -> Scoping {
        let mut before = Before {
            file: context.path.to_string_lossy().into_owned(),
            file_dir: context.path.parent().map(Path::to_path_buf).unwrap_or_default(),
            typescript: context.program.source_type.is_typescript(),
            source_text: context.source_text,
            errors: context.errors,
            doc_init_import: None,
            passes: self,
        };
        traverse_mut(&mut before, context.allocator, context.program, scoping, ())
    }

    fn after(&mut self, context: PassContext<'_, '_>, scoping: Scoping) -> Scoping {
        if let Some(package_imports) = &mut self.package_imports {
            package_imports.collect(context.program);
        }
        match &mut self.doc_init {
            Some(doc_init) if doc_init.has_calls() => doc_init.after(context, scoping),
            _ => scoping,
        }
    }
}

/// The plugins' visitors before the presets, merged into one traversal.
struct Before<'a, 'p> {
    passes: &'p mut TungstenPasses,
    file: String,
    file_dir: PathBuf,
    typescript: bool,
    source_text: &'a str,
    errors: &'p mut Vec<OxcDiagnostic>,
    doc_init_import: Option<BoundIdentifier<'a>>,
}

impl<'a> Traverse<'a, ()> for Before<'a, '_> {
    fn enter_program(&mut self, program: &mut Program<'a>, ctx: &mut TraverseCtx<'a>) {
        if self.passes.doc_init.is_some() {
            self.doc_init_import = DocInit::insert_import(program, self.source_text, ctx);
        }
    }

    fn exit_program(&mut self, _program: &mut Program<'a>, _ctx: &mut TraverseCtx<'a>) {
        if let Some(component_styles) = &self.passes.component_styles {
            component_styles.check_results(self.errors);
        }
    }

    fn enter_statement(&mut self, stmt: &mut Statement<'a>, ctx: &mut TraverseCtx<'a>) {
        match stmt {
            Statement::ImportDeclaration(decl) => {
                if self.passes.jsx_extensions_to_js {
                    jsx_extensions_to_js::rewrite(&mut decl.source, ctx);
                }
                if let Some(package_imports) = &mut self.passes.package_imports {
                    package_imports.visit_import(decl, ctx);
                }
            }
            Statement::ExportFromDeclaration(decl) => {
                if self.passes.dev_cache_bust {
                    dev_cache_bust::version(&mut decl.source, &self.file_dir, ctx);
                }
                return;
            }
            Statement::ExportAllDeclaration(decl) => {
                if self.passes.dev_cache_bust {
                    dev_cache_bust::version(&mut decl.source, &self.file_dir, ctx);
                }
                return;
            }
            _ => return,
        }
        if let Some(css_imports) = &self.passes.css_imports
            && let Some(source_start) =
                css_imports.transform(stmt, &self.file, self.typescript, self.errors, ctx)
        {
            // Babel stops visiting a replaced node, so later plugins never see this import
            if let Some(package_imports) = &mut self.passes.package_imports {
                package_imports.import_converted(source_start);
            }
            return;
        }
        if self.passes.dev_cache_bust
            && let Statement::ImportDeclaration(decl) = stmt
        {
            dev_cache_bust::version(&mut decl.source, &self.file_dir, ctx);
        }
    }

    fn enter_expression(&mut self, expr: &mut Expression<'a>, ctx: &mut TraverseCtx<'a>) {
        if let Some(component_styles) = &mut self.passes.component_styles {
            component_styles.enter_expression(expr, ctx);
        }
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, _ctx: &mut TraverseCtx<'a>) {
        if let Some(component_styles) = &mut self.passes.component_styles {
            component_styles.exit_expression(expr);
        }
    }

    fn enter_class_body(&mut self, _body: &mut ClassBody<'a>, _ctx: &mut TraverseCtx<'a>) {
        if let Some(component_styles) = &mut self.passes.component_styles {
            component_styles.enter_class_body();
        }
    }

    fn exit_class_body(&mut self, body: &mut ClassBody<'a>, ctx: &mut TraverseCtx<'a>) {
        if let Some(component_styles) = &mut self.passes.component_styles {
            component_styles.exit_class_body(body, ctx);
        }
    }

    fn enter_call_expression(&mut self, call: &mut CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
        if let (Some(doc_init), Some(import)) = (&mut self.passes.doc_init, &self.doc_init_import) {
            doc_init.claim_call(call, import, ctx);
        }
    }
}
