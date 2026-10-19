mod common;

use common::{compile_with, squash};
use serde_json::json;

const SOURCE: &str = r"export class Card {
    style = Scss`.box { color: red; }`
    render() { return Css`a ${1} b` }
}
const loose = Sass`
  p
    margin: 0
`
const zone = ReactZoneScss`div { }`
const ignored = [css`x`, Other`y`, obj.Css`z`, Css(`w`)]
";

#[test]
fn phase_one_lists_each_style_template_and_leaves_it_in_place() {
    let ret = compile_with("m.js", SOURCE, r#"{ "componentStyles": {} }"#);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert_eq!(
        ret.output["componentStyles"],
        json!([
            { "index": 1, "tag": "Scss", "raw": ".box { color: red; }", "inClass": true },
            { "index": 2, "tag": "Css", "raw": "a ", "inClass": true },
            { "index": 3, "tag": "Sass", "raw": "\n  p\n    margin: 0\n", "inClass": false },
            { "index": 4, "tag": "ReactZoneScss", "raw": "div { }", "inClass": false },
        ])
    );
    assert!(ret.code.contains("Scss`"), "{}", ret.code);
}

#[test]
fn skips_a_style_template_inside_another() {
    let source = "const outer = Css`a ${Css`inner`} b`;\nconst next = Css`next`;\n";
    let ret = compile_with("m.js", source, r#"{ "componentStyles": {} }"#);
    let tags: Vec<_> = ret.output["componentStyles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["raw"].clone())
        .collect();
    assert_eq!(tags, [json!("a "), json!("next")]);
}

#[test]
fn finds_the_nearest_class_body_through_functions_but_not_from_extends() {
    let source = "class A extends base(Css`e`) { m() { return () => Css`f` } }\nclass B { x = class { y = Css`g` } }\n";
    let ret = compile_with("m.js", source, r#"{ "componentStyles": {} }"#);
    let in_class: Vec<_> = ret.output["componentStyles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["inClass"].clone())
        .collect();
    assert_eq!(in_class, [json!(false), json!(true), json!(true)]);
}

/// Written out rather than built with `json!`, which would sort the names and hide their order.
fn result(module: &str) -> String {
    format!(
        r#"{{ "moduleName": "{module}", "tags": {{ "div": "{module}_div", "a": "{module}_a" }}, "classes": {{ "box": "{module}_box" }}, "keyframes": {{}}, "stylesheet": "i/{module}.css" }}"#
    )
}

fn results(modules: &[&str]) -> String {
    let results: Vec<_> = modules.iter().map(|module| result(module)).collect();
    format!(r#"{{ "componentStyles": {{ "results": [{}] }} }}"#, results.join(", "))
}

#[test]
fn phase_two_substitutes_the_style_objects_and_the_class_stylesheets() {
    let ret = compile_with("m.js", SOURCE, &results(&["m", "m-2", "m-3", "m-4"]));
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    let code = squash(&ret.code);
    assert!(
        code.contains(r#"style = { "moduleName": "m", "tags": { "div": "m_div", "a": "m_a" }, "classes": { "box": "m_box" }, "keyframes": {} };"#),
        "{code}"
    );
    assert!(code.contains(r#"return { "moduleName": "m-2","#), "{code}");
    assert!(code.contains(r#"const loose = { "moduleName": "m-3","#), "{code}");
    assert!(
        code.contains(r#"static $stylesheet = "i/m.css"; static $stylesheet = "i/m-2.css"; }"#),
        "{code}"
    );
    assert!(!code.contains("i/m-3.css") && !code.contains("i/m-4.css"), "{code}");
    assert!(!code.contains("Scss`") && code.contains("css`x`"), "{code}");
}

#[test]
fn phase_two_needs_a_result_per_template() {
    let ret = compile_with("m.js", SOURCE, &results(&["m"]));
    assert_eq!(ret.errors.len(), 1, "{:?}", ret.errors);
    assert!(ret.errors[0].contains("4 style templates but 1 result"), "{:?}", ret.errors);
}

#[test]
fn leaves_style_templates_alone_when_not_enabled() {
    let ret = compile_with("m.js", SOURCE, "{}");
    assert!(ret.code.contains("Scss`"), "{}", ret.code);
    assert!(ret.output.get("componentStyles").is_none(), "{}", ret.output);
}
