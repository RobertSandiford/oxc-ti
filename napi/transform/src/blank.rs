use std::path::Path;

use napi_derive::napi;

use oxc::span::SourceType;
use oxc_napi::OxcError;

#[napi(object)]
pub struct BlankTypesResult {
    /// The JavaScript, the same length as the source, line for line.
    pub code: String,
    pub errors: Vec<OxcError>,
}

/// TightScript: erase TypeScript and TightScript types by overwriting them with whitespace.
///
/// Lines and columns are unchanged, so the result needs no source map. Like Node's strip-only
/// mode, enums, namespaces with values, parameter properties, `import =` and `export =` are errors.
#[allow(clippy::needless_pass_by_value, clippy::allow_attributes)]
#[napi]
pub fn blank_types_sync(filename: String, source_text: String) -> BlankTypesResult {
    let source_type =
        SourceType::from_path(Path::new(&filename)).unwrap_or_default().with_typescript(true);
    let ret = oxc_ts_blank::blank(&source_text, source_type);
    BlankTypesResult {
        code: ret.code,
        errors: OxcError::from_diagnostics(&filename, &source_text, ret.errors),
    }
}
