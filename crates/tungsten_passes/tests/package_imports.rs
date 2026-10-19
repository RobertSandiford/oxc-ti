mod common;

use common::{compile_with, squash};
use serde_json::json;

#[test]
fn lists_the_bare_imports_left_after_compiling_including_the_jsx_runtime() {
    let source = "import { a } from 'pkg-a';\nimport b from './b.js';\nimport c from '../c.js';\nimport { d } from '@scope/d/sub';\nimport 'side-effect';\nimport { a as again } from 'pkg-a';\nexport const e = <div>{a}{b}{c}{d}{again}</div>;\n";
    let ret = compile_with("m.jsx", source, r#"{ "packageImports": {} }"#);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    let mut bare = ret.output["bareImports"].as_array().unwrap().clone();
    bare.sort_by_key(|value| value.as_str().unwrap().to_string());
    assert_eq!(
        serde_json::Value::Array(bare),
        json!(["@scope/d/sub", "pkg-a", "side-effect", "tungsten/jsx-runtime"])
    );
}

#[test]
fn leaves_out_imports_typescript_removes() {
    let source = "import type { T } from 'types-only';\nimport { U } from 'used-as-type';\nimport { v } from 'value';\nexport const x: T | U = v;\n";
    let ret = compile_with("m.ts", source, r#"{ "packageImports": {} }"#);
    assert_eq!(ret.output["bareImports"], json!(["value"]));
}

#[test]
fn applies_replacements_and_still_lists_the_original_specifier() {
    let source = "import 'pkg/theme.css';\nimport { x } from 'pkg';\nexport { x };\n";
    let options = r#"{ "packageImports": { "replacements": { "pkg/theme.css": "/.packages/pkg@1.2.3/theme.css" } } }"#;
    let ret = compile_with("m.js", source, options);
    assert!(ret.code.contains("import \"/.packages/pkg@1.2.3/theme.css\";"), "{}", ret.code);
    assert!(ret.code.contains("from \"pkg\""), "{}", ret.code);
    assert_eq!(ret.output["bareImports"], json!(["pkg/theme.css", "pkg"]));
}

#[test]
fn a_replaced_stylesheet_import_becomes_its_web_location() {
    let source = "import theme from 'pkg/theme.scss';\nexport { theme };\n";
    let options = json!({
        "packageImports": { "replacements": { "pkg/theme.scss": "/.packages/pkg@1.2.3/theme.scss" } },
        "cssImports": { "absCodeDir": "/app/code", "absSourceDir": "/app/src" },
    });
    let ret = compile_with("/app/code/m.js", source, &options.to_string());
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(
        squash(&ret.code).contains("const theme = \"/.packages/pkg@1.2.3/theme.css\";"),
        "{}",
        ret.code
    );
    assert_eq!(ret.output["bareImports"], json!(["pkg/theme.scss"]));
}

#[test]
fn lists_the_specifier_jsx_extensions_to_js_produced() {
    let source = "import { a } from 'pkg/a.tsx';\nexport { a };\n";
    let ret =
        compile_with("m.js", source, r#"{ "jsxExtensionsToJs": true, "packageImports": {} }"#);
    assert_eq!(ret.output["bareImports"], json!(["pkg/a.js"]));
}

#[test]
fn has_no_list_when_not_enabled() {
    let ret = compile_with("m.js", "import { a } from 'pkg';\nexport { a };\n", "{}");
    assert!(ret.output.get("bareImports").is_none(), "{}", ret.output);
}
