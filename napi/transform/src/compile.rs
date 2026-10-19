use std::path::Path;

use napi_derive::napi;

use oxc::diagnostics::OxcDiagnostic;
use oxc_napi::OxcError;
use oxc_ti_compile::{CompileOptions, NoPasses, Passes, compile};
use tungsten_passes::TungstenPasses;

#[napi(object)]
#[derive(Default)]
pub struct TightScriptCompileOptions {
    /// The module JSX compiles to imports from, `<source>/jsx-runtime`. Default: `react`.
    pub jsx_import_source: Option<String>,
    /// Default: `false`.
    pub source_map: Option<bool>,
    /// Options for tungsten's passes as JSON; the shapes are documented in the `tungsten_passes` crate.
    pub tungsten: Option<String>,
}

#[napi(object)]
pub struct TightScriptCompileResult {
    pub code: String,
    /// The source map as JSON.
    pub map: Option<String>,
    pub errors: Vec<OxcError>,
    /// What tungsten's passes collected, as JSON, when `tungsten` options were given.
    pub tungsten: Option<String>,
}

/// TightScript: compile one module for the browser. Tungsten's passes run if asked, types are
/// removed and JSX is compiled to the automatic runtime; nothing is lowered.
#[allow(clippy::needless_pass_by_value, clippy::allow_attributes)]
#[napi]
pub fn compile_sync(
    filename: String,
    source_text: String,
    options: Option<TightScriptCompileOptions>,
) -> TightScriptCompileResult {
    let options = options.unwrap_or_default();
    let mut tungsten = match options.tungsten.as_deref().map(TungstenPasses::from_options_json) {
        Some(Ok(passes)) => Some(passes),
        Some(Err(message)) => {
            let error = OxcDiagnostic::error(format!("Invalid tungsten options: {message}"));
            return TightScriptCompileResult {
                code: String::new(),
                map: None,
                errors: OxcError::from_diagnostics(&filename, &source_text, vec![error]),
                tungsten: None,
            };
        }
        None => None,
    };
    let compile_options = CompileOptions {
        jsx_import_source: options.jsx_import_source,
        source_map: options.source_map.unwrap_or(false),
    };
    let passes: &mut dyn Passes = match &mut tungsten {
        Some(passes) => passes,
        None => &mut NoPasses,
    };
    let ret = compile(Path::new(&filename), &source_text, &compile_options, passes);
    TightScriptCompileResult {
        code: ret.code,
        map: ret.map,
        errors: OxcError::from_diagnostics(&filename, &source_text, ret.errors),
        tungsten: tungsten.map(|passes| passes.output_json()),
    }
}
