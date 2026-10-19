mod common;

use common::compile_with;

const OPTIONS: &str = r#"{ "jsxExtensionsToJs": true }"#;

#[test]
fn rewrites_jsx_tsx_and_ts_import_sources_to_js() {
    let source = "import a from './a.jsx';\nimport b from './b.tsx';\nimport c from '../c.ts';\nimport d from 'pkg/d.tsx';\nconsole.log(a, b, c, d);\n";
    let ret = compile_with("m.js", source, OPTIONS);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    for expected in ["\"./a.js\"", "\"./b.js\"", "\"../c.js\"", "\"pkg/d.js\""] {
        assert!(ret.code.contains(expected), "missing {expected} in {}", ret.code);
    }
}

#[test]
fn leaves_other_import_sources_alone() {
    let source = "import a from './a.js';\nimport s from './s.css';\nimport p from 'pkg';\nimport m from './m.mts';\nconsole.log(a, s, p, m);\n";
    let ret = compile_with("m.js", source, OPTIONS);
    for expected in ["\"./a.js\"", "\"./s.css\"", "\"pkg\"", "\"./m.mts\""] {
        assert!(ret.code.contains(expected), "missing {expected} in {}", ret.code);
    }
}

#[test]
fn only_rewrites_import_declarations() {
    let source =
        "export { x } from './x.jsx';\nexport * from './y.tsx';\nconst z = import('./z.jsx');\n";
    let ret = compile_with("m.js", source, OPTIONS);
    for expected in ["\"./x.jsx\"", "\"./y.tsx\"", "\"./z.jsx\""] {
        assert!(ret.code.contains(expected), "missing {expected} in {}", ret.code);
    }
}

#[test]
fn does_nothing_when_not_enabled() {
    let ret = compile_with("m.js", "import a from './a.jsx';\nconsole.log(a);\n", "{}");
    assert!(ret.code.contains("\"./a.jsx\""), "{}", ret.code);
}
