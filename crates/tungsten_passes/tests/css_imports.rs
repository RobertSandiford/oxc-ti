mod common;

use std::{fs, path::Path};

use common::{compile_with, squash};

struct App {
    _dir: tempfile::TempDir,
    code_dir: String,
    options: String,
}

/// A code dir to compile from and a source dir holding the stylesheets, both under one temp dir.
fn app(stylesheets: &[&str]) -> App {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_string_lossy().split('\\').collect::<Vec<_>>().join("/");
    let code_dir = format!("{root}/code");
    let source_dir = format!("{root}/src");
    for stylesheet in stylesheets {
        let path = Path::new(&source_dir).join(stylesheet);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }
    let options =
        serde_json::json!({ "cssImports": { "absCodeDir": code_dir, "absSourceDir": source_dir } })
            .to_string();
    App { _dir: dir, code_dir, options }
}

#[test]
fn replaces_a_stylesheet_import_with_its_code_dir_path_as_css() {
    let app = app(&["pages/g.scss", "x.css", "a.sass"]);
    let source = "import g from './g.scss';\nimport x from '../x.css';\nexport { g, x };\n";
    let ret = compile_with(format!("{}/pages/Home.jsx", app.code_dir), source, &app.options);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    let code = squash(&ret.code);
    assert!(code.contains("const g = \"/pages/g.css\";"), "{code}");
    // Babel joins `..` on without normalising it
    assert!(code.contains("const x = \"/pages/../x.css\";"), "{code}");
    assert!(!code.contains("import"), "{code}");

    let ret = compile_with(
        format!("{}/Root.jsx", app.code_dir),
        "import a from './a.sass';\nexport { a };\n",
        &app.options,
    );
    assert!(squash(&ret.code).contains("const a = \"/a.css\";"), "{}", ret.code);
}

#[test]
fn takes_the_local_name_of_any_single_specifier() {
    let app = app(&["s.css"]);
    let source =
        "import { sheet } from './s.css';\nimport * as ns from './s.css';\nexport { sheet, ns };\n";
    let ret = compile_with(format!("{}/m.js", app.code_dir), source, &app.options);
    let code = squash(&ret.code);
    assert!(code.contains("const sheet = \"/s.css\";"), "{code}");
    assert!(code.contains("const ns = \"/s.css\";"), "{code}");
}

#[test]
fn a_missing_stylesheet_is_an_error() {
    let app = app(&[]);
    let ret = compile_with(
        format!("{}/m.js", app.code_dir),
        "import n from './nope.css';\nexport { n };\n",
        &app.options,
    );
    assert_eq!(ret.errors, ["Could not find stylesheet './nope.css'"]);
}

#[test]
fn a_bare_stylesheet_import_is_not_checked() {
    let app = app(&[]);
    let source = "import t from 'pkg/theme.scss';\nimport u from '/abs/u.css';\nexport { t, u };\n";
    let ret = compile_with(format!("{}/m.js", app.code_dir), source, &app.options);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    let code = squash(&ret.code);
    assert!(code.contains("const t = \"pkg/theme.css\";"), "{code}");
    assert!(code.contains("const u = \"/abs/u.css\";"), "{code}");
}

#[test]
fn no_specifier_or_several_warns_and_keeps_the_import() {
    let app = app(&["s.css"]);
    let source = "import './s.css';\nimport a, { b } from './s.css';\nexport { a, b };\n";
    let ret = compile_with(format!("{}/m.js", app.code_dir), source, &app.options);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert_eq!(ret.warnings.len(), 2, "{:?}", ret.warnings);
    assert!(ret.warnings[0].contains("is not being imported to a variable"), "{:?}", ret.warnings);
    assert!(ret.warnings[1].contains("has too any specifiers"), "{:?}", ret.warnings);
    let code = squash(&ret.code);
    assert!(code.contains("import \"./s.css\";"), "{code}");
    assert!(code.contains("import a, { b } from \"./s.css\";"), "{code}");
}

#[test]
fn an_import_typescript_elides_is_left_for_it_to_remove() {
    let app = app(&[]);
    let source = "import unused from './missing.css';\nimport type T from './types.css';\nexport const n: number = 1;\n";
    let ret = compile_with(format!("{}/m.ts", app.code_dir), source, &app.options);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(!ret.code.contains("css"), "{}", ret.code);
}

#[test]
fn a_typescript_import_used_as_a_value_is_replaced() {
    let app = app(&["s.css"]);
    let source = "import s from './s.css';\nexport const sheet: string = s;\n";
    let ret = compile_with(format!("{}/m.tsx", app.code_dir), source, &app.options);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(squash(&ret.code).contains("const s = \"/s.css\";"), "{}", ret.code);
}

#[test]
fn leaves_stylesheet_imports_alone_when_not_enabled() {
    let ret = compile_with("m.js", "import g from './g.scss';\nexport { g };\n", "{}");
    assert!(ret.code.contains("import g from \"./g.scss\""), "{}", ret.code);
}
