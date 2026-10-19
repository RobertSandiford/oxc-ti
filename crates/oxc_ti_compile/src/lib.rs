//! Compiles one TightScript, TypeScript or JavaScript module into a browser ES module.
//!
//! Like a Babel pipeline: custom passes see the parsed program first, with its types and JSX intact,
//! then types are removed and JSX is compiled to the automatic runtime, then custom passes may run
//! again (as a Babel plugin's `Program.exit` would), then code and a source map are printed.

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;
use oxc_transformer::{JsxRuntime, TransformOptions, Transformer};

pub struct CompileOptions {
    /// The module JSX compiles to imports from, `<source>/jsx-runtime`.
    pub jsx_import_source: Option<String>,
    pub source_map: bool,
}

pub struct CompileResult {
    pub code: String,
    /// The source map as JSON.
    pub map: Option<String>,
    pub errors: Vec<OxcDiagnostic>,
}

/// What a pass works on. Errors it reports are returned with the result.
pub struct PassContext<'a, 'c> {
    pub allocator: &'a Allocator,
    pub program: &'c mut Program<'a>,
    pub source_text: &'a str,
    pub path: &'c Path,
    pub errors: &'c mut Vec<OxcDiagnostic>,
}

/// Custom transforms. A pass keeps any data it collects for the caller to read after `compile`.
pub trait Passes {
    /// Runs on the parsed program, with its types and JSX intact.
    fn before(&mut self, _context: PassContext<'_, '_>, scoping: Scoping) -> Scoping {
        scoping
    }

    /// Runs after types are removed and JSX is compiled.
    fn after(&mut self, _context: PassContext<'_, '_>, scoping: Scoping) -> Scoping {
        scoping
    }
}

/// No custom transforms.
pub struct NoPasses;

impl Passes for NoPasses {}

pub fn compile(
    path: &Path,
    source_text: &str,
    options: &CompileOptions,
    passes: &mut dyn Passes,
) -> CompileResult {
    let allocator = Allocator::default();
    let source_text = allocator.alloc_str(source_text);
    let mut source_type = SourceType::from_path(path).unwrap_or_default();
    // Like Babel's React preset, JSX is allowed in plain .js; .ts keeps `<T>x` as a type assertion.
    if source_type.is_javascript() {
        source_type = source_type.with_jsx(true);
    }
    let parsed = Parser::new(&allocator, source_text, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return CompileResult {
            code: String::new(),
            map: None,
            errors: parsed.diagnostics.to_vec(),
        };
    }
    let mut program = parsed.program;
    let mut errors = vec![];

    let semantic = SemanticBuilder::new().with_excess_capacity(2.0).build(&program);
    let mut scoping = semantic.semantic.into_scoping();
    let context = PassContext {
        allocator: &allocator,
        program: &mut program,
        source_text,
        path,
        errors: &mut errors,
    };
    scoping = passes.before(context, scoping);

    // The default lowers nothing: the dev server's browsers run current JavaScript.
    let mut transform_options = TransformOptions::default();
    transform_options.jsx.runtime = JsxRuntime::Automatic;
    transform_options.jsx.import_source.clone_from(&options.jsx_import_source);
    transform_options.typescript.allow_declare_fields = true;
    let transformed = Transformer::new(&allocator, path, &transform_options)
        .build_with_scoping(scoping, &mut program);
    errors.extend(transformed.diagnostics);

    let context = PassContext {
        allocator: &allocator,
        program: &mut program,
        source_text,
        path,
        errors: &mut errors,
    };
    passes.after(context, transformed.scoping);

    let codegen_options = CodegenOptions {
        source_map_path: options.source_map.then(|| path.to_path_buf()),
        ..CodegenOptions::default()
    };
    let printed = Codegen::new().with_options(codegen_options).build(&program);
    CompileResult { code: printed.code, map: printed.map.map(|map| map.to_json_string()), errors }
}
