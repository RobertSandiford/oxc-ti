mod common;

use std::{
    fs::{self, File},
    path::Path,
    time::{Duration, SystemTime},
};

use common::compile_with;

const OPTIONS: &str = r#"{ "devCacheBust": true }"#;

/// Writes `name` under `dir` with a modification time `secs` seconds and `nanos` nanoseconds after the epoch.
fn file_with_mtime(dir: &Path, name: &str, secs: u64, nanos: u32) {
    let path = dir.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = File::create(&path).unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::new(secs, nanos)).unwrap();
}

#[test]
fn versions_relative_imports_by_the_target_mtime_as_node_prints_mtime_ms() {
    let dir = tempfile::tempdir().unwrap();
    file_with_mtime(dir.path(), "a.js", 1_700_000_000, 123_456_700);
    file_with_mtime(dir.path(), "whole.js", 1_700_000_000, 0);
    let source = "import a from './a.js';\nimport w from './whole.js';\nconsole.log(a, w);\n";
    let ret = compile_with(dir.path().join("m.js"), source, OPTIONS);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    // node: `String(1700000000 * 1e3 + 123456700 / 1e6)`
    assert!(ret.code.contains("\"./a.js?v=1700000000123.4568\""), "{}", ret.code);
    assert!(ret.code.contains("\"./whole.js?v=1700000000000\""), "{}", ret.code);
}

#[test]
fn prints_a_tied_mtime_as_javascript_does_rounding_to_even() {
    let dir = tempfile::tempdir().unwrap();
    // exactly 1792235402941.78125 ms: .7812 and .7813 both read back, and JavaScript picks the even one
    file_with_mtime(dir.path(), "t.js", 1_792_235_402, 941_781_200);
    let ret = compile_with(
        dir.path().join("m.js"),
        "import t from './t.js';\nconsole.log(t);\n",
        OPTIONS,
    );
    assert!(ret.code.contains("\"./t.js?v=1792235402941.7812\""), "{}", ret.code);
}

#[test]
fn a_js_specifier_tries_js_jsx_ts_tsx_in_that_order() {
    let dir = tempfile::tempdir().unwrap();
    file_with_mtime(dir.path(), "b.jsx", 1_000, 0);
    file_with_mtime(dir.path(), "b.tsx", 2_000, 0);
    file_with_mtime(dir.path(), "c.tsx", 3_000, 0);
    file_with_mtime(dir.path(), "sub/d.ts", 4_000, 0);
    let source = "import b from './b.js';\nimport c from './c.js';\nimport d from './sub/d.js';\nconsole.log(b, c, d);\n";
    let ret = compile_with(dir.path().join("m.js"), source, OPTIONS);
    assert!(ret.code.contains("\"./b.js?v=1000000\""), "{}", ret.code);
    assert!(ret.code.contains("\"./c.js?v=3000000\""), "{}", ret.code);
    assert!(ret.code.contains("\"./sub/d.js?v=4000000\""), "{}", ret.code);
}

#[test]
fn jsx_ts_and_tsx_specifiers_have_their_own_candidates() {
    let dir = tempfile::tempdir().unwrap();
    file_with_mtime(dir.path(), "e.js", 5_000, 0);
    file_with_mtime(dir.path(), "f.ts", 6_000, 0);
    file_with_mtime(dir.path(), "g.js", 7_000, 0);
    // `.jsx` falls back to `.js`; `.ts` and `.tsx` only ever name themselves
    let source = "import e from './e.jsx';\nimport f from './f.ts';\nimport g from './g.ts';\nconsole.log(e, f, g);\n";
    let ret = compile_with(dir.path().join("m.js"), source, OPTIONS);
    assert!(ret.code.contains("\"./e.jsx?v=5000000\""), "{}", ret.code);
    assert!(ret.code.contains("\"./f.ts?v=6000000\""), "{}", ret.code);
    assert!(ret.code.contains("\"./g.ts\""), "{}", ret.code);
}

#[test]
fn leaves_bare_missing_and_extensionless_specifiers_alone() {
    let dir = tempfile::tempdir().unwrap();
    file_with_mtime(dir.path(), "h.js", 8_000, 0);
    let source = "import p from 'pkg/h.js';\nimport m from './missing.js';\nimport h from './h';\nimport q from './h.js?x=1';\nconsole.log(p, m, h, q);\n";
    let ret = compile_with(dir.path().join("m.js"), source, OPTIONS);
    for expected in ["\"pkg/h.js\"", "\"./missing.js\"", "\"./h\"", "\"./h.js?x=1\""] {
        assert!(ret.code.contains(expected), "missing {expected} in {}", ret.code);
    }
}

#[test]
fn versions_export_from_and_export_all_too() {
    let dir = tempfile::tempdir().unwrap();
    file_with_mtime(dir.path(), "x.js", 9_000, 0);
    let source =
        "export { x } from './x.js';\nexport * from './x.js';\nexport * as ns from './x.js';\n";
    let ret = compile_with(dir.path().join("m.js"), source, OPTIONS);
    assert_eq!(ret.code.matches("\"./x.js?v=9000000\"").count(), 3, "{}", ret.code);
}

#[test]
fn sees_the_specifier_jsx_extensions_to_js_produced() {
    let dir = tempfile::tempdir().unwrap();
    file_with_mtime(dir.path(), "k.jsx", 10_000, 0);
    let source = "import k from './k.jsx';\nconsole.log(k);\n";
    let options = r#"{ "jsxExtensionsToJs": true, "devCacheBust": true }"#;
    let ret = compile_with(dir.path().join("m.js"), source, options);
    assert!(ret.code.contains("\"./k.js?v=10000000\""), "{}", ret.code);
}

#[test]
fn leaves_relative_specifiers_alone_when_not_enabled() {
    let dir = tempfile::tempdir().unwrap();
    file_with_mtime(dir.path(), "a.js", 1_000, 0);
    let ret =
        compile_with(dir.path().join("m.js"), "import a from './a.js';\nconsole.log(a);\n", "{}");
    assert!(ret.code.contains("\"./a.js\""), "{}", ret.code);
}
