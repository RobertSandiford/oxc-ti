#![allow(dead_code)]

use std::path::Path;

use oxc_ti_compile::{CompileOptions, compile};
use serde_json::Value;
use tungsten_passes::TungstenPasses;

pub struct Compiled {
    pub code: String,
    pub output: Value,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn compile_with(path: impl AsRef<Path>, source: &str, options_json: &str) -> Compiled {
    let mut passes = TungstenPasses::from_options_json(options_json).expect("valid options");
    let options =
        CompileOptions { jsx_import_source: Some("tungsten".to_string()), source_map: false };
    let ret = compile(path.as_ref(), source, &options, &mut passes);
    let (warnings, errors): (Vec<_>, Vec<_>) =
        ret.errors.iter().partition(|error| error.severity == oxc_diagnostics::Severity::Warning);
    Compiled {
        code: ret.code,
        output: serde_json::from_str(&passes.output_json()).expect("output is JSON"),
        errors: errors.iter().map(|error| error.message.to_string()).collect(),
        warnings: warnings.iter().map(|error| error.message.to_string()).collect(),
    }
}

/// Collapses runs of whitespace, so assertions need not match codegen's line breaks.
pub fn squash(code: &str) -> String {
    code.split_whitespace().collect::<Vec<_>>().join(" ")
}
