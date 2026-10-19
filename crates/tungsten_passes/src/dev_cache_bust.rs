//! `BabelAddDevCacheBust`: a relative specifier gets `?v=<mtimeMs>` of the file it names.

use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use oxc_ast::ast::StringLiteral;
use oxc_str::Str;

use crate::TraverseCtx;

pub fn version<'a>(source: &mut StringLiteral<'a>, file_dir: &Path, ctx: &TraverseCtx<'a>) {
    let Some(specifier) = source.value.as_str() else { return };
    let Some(mtime_ms) = on_disk_mtime_ms(file_dir, specifier) else { return };
    source.value = Str::from_str_in(&versioned(specifier, mtime_ms), ctx).into();
    source.raw = None;
}

/// The files a specifier may name in the code dir: a `.js` one may still be `.jsx`, `.ts` or `.tsx`.
#[expect(clippy::case_sensitive_file_extension_comparisons, reason = "the plugin's `endsWith` is")]
fn candidates(specifier: &str) -> Vec<String> {
    if let Some(stem) = specifier.strip_suffix(".jsx") {
        vec![specifier.to_string(), format!("{stem}.js")]
    } else if let Some(stem) = specifier.strip_suffix(".js") {
        [".js", ".jsx", ".ts", ".tsx"]
            .iter()
            .map(|extension| format!("{stem}{extension}"))
            .collect()
    } else if specifier.ends_with(".ts") || specifier.ends_with(".tsx") {
        vec![specifier.to_string()]
    } else {
        vec![]
    }
}

fn on_disk_mtime_ms(file_dir: &Path, specifier: &str) -> Option<f64> {
    if !specifier.starts_with('.') {
        return None;
    }
    candidates(specifier).iter().find_map(|candidate| {
        let modified = fs::metadata(file_dir.join(candidate)).ok()?.modified().ok()?;
        Some(node_mtime_ms(modified))
    })
}

/// Node's `stats.mtimeMs`: `seconds * 1e3 + nanoseconds / 1e6`, in doubles.
#[expect(
    clippy::cast_precision_loss,
    clippy::suboptimal_flops,
    reason = "Node's arithmetic exactly: a fused multiply-add would round differently"
)]
fn node_mtime_ms(time: SystemTime) -> f64 {
    let since_epoch = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    since_epoch.as_secs() as f64 * 1e3 + f64::from(since_epoch.subsec_nanos()) / 1e6
}

/// The number as JavaScript prints it: Rust's `{}` breaks a tie between two shortest forms the other way.
fn versioned(specifier: &str, mtime_ms: f64) -> String {
    let separator = if specifier.contains('?') { '&' } else { '?' };
    let mut buffer = dragonbox_ecma::Buffer::new();
    format!("{specifier}{separator}v={}", buffer.format(mtime_ms))
}

#[cfg(test)]
mod test {
    use super::versioned;

    #[test]
    fn appends_to_an_existing_query_with_an_ampersand() {
        assert_eq!(versioned("./a?x.js", 1.5), "./a?x.js&v=1.5");
        assert_eq!(versioned("./a.js", 1_700_000_000_000.0), "./a.js?v=1700000000000");
    }
}
