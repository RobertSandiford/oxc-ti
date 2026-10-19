//! `BabelTransformJsxExtensionsToJs`: an import of `.jsx`, `.tsx` or `.ts` names the compiled `.js`.

use oxc_ast::ast::StringLiteral;
use oxc_str::Str;

use crate::TraverseCtx;

pub fn rewrite<'a>(source: &mut StringLiteral<'a>, ctx: &TraverseCtx<'a>) {
    let Some(location) = source.value.as_str() else { return };
    let stem = if let Some(stem) = location.strip_suffix(".jsx") {
        stem
    } else if let Some(stem) = location.strip_suffix(".tsx") {
        stem
    } else if let Some(stem) = location.strip_suffix(".ts") {
        stem
    } else {
        return;
    };
    source.value = Str::from_strs_array_in([stem, ".js"], ctx).into();
    source.raw = None;
}
