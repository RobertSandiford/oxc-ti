mod common;

use common::{Compiled, compile_with, squash};
use serde_json::json;

fn doc_init(path: &str, source: &str) -> Compiled {
    compile_with(path, source, r#"{ "docInit": { "baseId": 0 } }"#)
}

const THEME: &str = "export const theme = DocInit(() => {
  document.documentElement.dataset.theme = localStorage.getItem('theme') ?? 'light'
  return document.documentElement.dataset.theme
}, 'light')
";

#[test]
fn rewrites_the_call_imports_the_runtime_and_outputs_the_entry() {
    let ret = doc_init("Root.js", THEME);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    let code = squash(&ret.code);
    assert!(code.starts_with("import { docInit as _docInit } from \"tungsten\";"), "{code}");
    assert!(code.contains("export const theme = _docInit(0, \"light\");"), "{code}");
    assert!(!code.contains("localStorage"), "{code}");

    let entries = ret.output["docInit"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0]["id"], json!(0));
    assert_eq!((&entries[0]["line"], &entries[0]["column"]), (&json!(1), &json!(30)));
    let function = squash(entries[0]["function"].as_str().unwrap());
    assert!(function.starts_with("() => {"), "{function}");
    assert!(function.contains("localStorage.getItem(\"theme\") ?? \"light\""), "{function}");
}

#[test]
fn numbers_calls_from_the_base_id_in_source_order_with_utf16_columns() {
    let source = "export const a = DocInit(() => 'ok', '')\n\nexport const b = DocInit(() => { throw new Error('boom') }, '')\n/* é😀 */ export const c = DocInit(() => 'not reached')";
    let ret = compile_with("A.js", source, r#"{ "docInit": { "baseId": 7 } }"#);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    let code = squash(&ret.code);
    assert!(code.contains("export const a = _docInit(7, \"\");"), "{code}");
    assert!(code.contains("export const b = _docInit(8, \"\");"), "{code}");
    assert!(code.contains("export const c = _docInit(9);"), "{code}");
    let places: Vec<_> = ret.output["docInit"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| (entry["id"].clone(), entry["line"].clone(), entry["column"].clone()))
        .collect();
    assert_eq!(
        places,
        [
            (json!(7), json!(1), json!(26)),
            (json!(8), json!(3), json!(26)),
            (json!(9), json!(4), json!(36))
        ]
    );
}

#[test]
fn leaves_a_file_without_a_global_call_untouched() {
    let ret = doc_init("Root.js", "export const x = init(() => 1);\n");
    assert_eq!(ret.code, "export const x = init(() => 1);\n");
    assert_eq!(ret.output["docInit"], json!([]));

    let source = "const DocInit = (fn) => fn();\nexport const x = DocInit(() => 1);\n";
    let ret = doc_init("Root.js", source);
    assert_eq!(ret.code, source);
    assert_eq!(ret.output["docInit"], json!([]));
}

#[test]
fn picks_an_import_name_that_does_not_collide() {
    let ret = doc_init(
        "Root.js",
        "const _docInit = 1;\nexport const x = DocInit(() => _docInit2, _docInit);\n",
    );
    let code = squash(&ret.code);
    assert!(code.contains("import { docInit as _docInit3 } from \"tungsten\";"), "{code}");
    assert!(code.contains("_docInit3(0, _docInit)"), "{code}");
}

#[test]
fn outputs_the_compiled_function_and_keeps_the_import_through_typescript() {
    let source =
        "export const n = DocInit((): number => { const x: number = 1; return x as number }, 0)\n";
    let ret = doc_init("Root.ts", source);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    let code = squash(&ret.code);
    assert!(code.contains("import { docInit as _docInit } from \"tungsten\";"), "{code}");
    assert!(code.contains("_docInit(0, 0)"), "{code}");
    let function = squash(ret.output["docInit"][0]["function"].as_str().unwrap());
    assert_eq!(function, "() => { const x = 1; return x; }");
}

fn first_error(source: &str) -> String {
    let ret = doc_init("Root.js", source);
    assert_eq!(ret.errors.len(), 1, "{:?}", ret.errors);
    ret.errors[0].clone()
}

#[test]
fn rejects_what_cannot_run_in_the_head() {
    assert!(
        first_error("const fn = () => 1\nexport const x = DocInit(fn, 0)")
            .contains("inline function expression")
    );
    assert!(first_error("export const x = DocInit()").contains("inline function expression"));
    assert!(first_error("export const x = DocInit(async () => 1, 0)").contains("cannot be async"));
    assert!(
        first_error("export const x = DocInit(function* () {}, 0)")
            .contains("cannot be async or a generator")
    );
    assert!(first_error("export const x = DocInit(a => a, 0)").contains("takes no parameters"));
    assert!(first_error("export const x = DocInit(() => 1, 0, 2)").contains("nothing more"));
}

#[test]
fn rejects_references_to_bindings_outside_the_function() {
    let error = first_error(
        "const key = 'theme'\nexport const x = DocInit(() => localStorage.getItem(key), '')",
    );
    assert!(error.contains("references 'key' from outside the function"), "{error}");
    let error = first_error("let n = 0\nexport const x = DocInit(() => { n++ })");
    assert!(error.contains("references 'n'"), "{error}");
}

#[test]
fn rejects_jsx_whose_runtime_import_the_head_lacks() {
    // Babel misses this (its preset's `_jsx` import has no scope binding) and emits a head script that throws
    let ret = doc_init("Root.jsx", "export const x = DocInit(() => <b />)");
    assert_eq!(ret.errors.len(), 1, "{:?}", ret.errors);
    assert!(ret.errors[0].contains("references '_jsx'"), "{:?}", ret.errors);
}

#[test]
fn accepts_globals_its_own_locals_and_plain_assignments_out() {
    let ok = |source: &str| {
        let ret = doc_init("Root.js", source);
        assert!(ret.errors.is_empty(), "{source}: {:?}", ret.errors);
    };
    ok(
        "export const x = DocInit(() => { const k = 'theme'; const read = () => localStorage.getItem(k); return read() ?? 'light' }, '')",
    );
    ok("export const x = DocInit(function named() { return named.length + arguments.length })");
    // Babel's ReferencedIdentifier skips assignment targets
    ok("let out\nexport const x = DocInit(() => { out = 1; [out] = [2]; ({ out } = { out: 3 }) })");
}

#[test]
fn rejects_this_from_the_enclosing_scope_but_allows_a_nested_plain_function() {
    let error = first_error("class Root { data = DocInit(() => this.x, 0) }");
    assert!(error.contains("'this' in a DocInit() function"), "{error}");
    let ok = |source: &str| {
        let ret = doc_init("Root.js", source);
        assert!(ret.errors.is_empty(), "{source}: {:?}", ret.errors);
    };
    ok("export const x = DocInit(function () { return this }, 0)");
    ok("export const x = DocInit(() => [1].map(function () { return this }), 0)");
    ok("export const x = DocInit(() => { class C { m() { return this } } return C })");
}

#[test]
fn does_nothing_when_not_enabled() {
    let ret = compile_with("Root.js", THEME, "{}");
    assert!(ret.code.contains("DocInit(() =>"), "{}", ret.code);
    assert!(ret.output.get("docInit").is_none(), "{}", ret.output);
}
