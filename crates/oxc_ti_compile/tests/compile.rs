use std::path::Path;

use oxc_ast::ast::{IdentifierReference, JSXElement, TSTypeAnnotation};
use oxc_semantic::Scoping;
use oxc_ti_compile::{CompileOptions, NoPasses, PassContext, Passes, compile};
use oxc_traverse::{Traverse, TraverseCtx, traverse_mut};

fn options() -> CompileOptions {
    CompileOptions { jsx_import_source: Some("tungsten".to_string()), source_map: true }
}

#[test]
fn removes_tightscript_types() {
    let source = "export class M { read x: number = 1; rw y = 2 }\nexport const v: owned rw { a: string } = { a: '' } as exact { a: string };\n";
    let ret = compile(Path::new("m.ts"), source, &options(), &mut NoPasses);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(!ret.code.contains("read") && !ret.code.contains("owned"), "{}", ret.code);
    assert!(ret.code.contains("x = 1") && ret.code.contains("y = 2"), "{}", ret.code);
}

#[test]
fn compiles_jsx_to_the_automatic_runtime() {
    let source = "export const App = (p: { n: number }) => <div class=\"a\">{p.n}</div>;\n";
    let ret = compile(Path::new("app.tsx"), source, &options(), &mut NoPasses);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(ret.code.contains("\"tungsten/jsx-runtime\""), "{}", ret.code);
    assert!(!ret.code.contains("<div"), "{}", ret.code);
}

#[test]
fn accepts_jsx_in_js_files_as_babel_does() {
    let ret = compile(Path::new("app.js"), "export const e = <b />;\n", &options(), &mut NoPasses);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(ret.code.contains("\"tungsten/jsx-runtime\""), "{}", ret.code);
}

#[test]
fn produces_a_source_map() {
    let ret =
        compile(Path::new("m.ts"), "export const a: number = 1;\n", &options(), &mut NoPasses);
    let map = ret.map.expect("a source map");
    assert!(map.contains("\"sources\":[\"m.ts\"]"), "{map}");
    let no_map = CompileOptions { source_map: false, ..options() };
    assert!(compile(Path::new("m.ts"), "1;", &no_map, &mut NoPasses).map.is_none());
}

#[test]
fn reports_parse_errors() {
    let ret = compile(Path::new("m.ts"), "let x: = 1;", &options(), &mut NoPasses);
    assert_eq!(ret.errors.len(), 1, "{:?}", ret.errors);
}

/// Renames `before` to `after` and records what it saw: type annotations only exist before the
/// TypeScript transform, and JSX elements only before the JSX transform.
#[derive(Default)]
struct Probe {
    saw_types: bool,
    saw_jsx: bool,
}

impl<'a> Traverse<'a, ()> for Probe {
    fn enter_identifier_reference(
        &mut self,
        it: &mut IdentifierReference<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        if it.name == "before" {
            it.name = "after".into();
        }
    }

    fn enter_ts_type_annotation(
        &mut self,
        _it: &mut TSTypeAnnotation<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.saw_types = true;
    }

    fn enter_jsx_element(&mut self, _it: &mut JSXElement<'a>, _ctx: &mut TraverseCtx<'a, ()>) {
        self.saw_jsx = true;
    }
}

#[derive(Default)]
struct BeforePass(Probe);

impl Passes for BeforePass {
    fn before(&mut self, context: PassContext<'_, '_>, scoping: Scoping) -> Scoping {
        traverse_mut(&mut self.0, context.allocator, context.program, scoping, ())
    }
}

#[derive(Default)]
struct AfterPass(Probe);

impl Passes for AfterPass {
    fn after(&mut self, context: PassContext<'_, '_>, scoping: Scoping) -> Scoping {
        traverse_mut(&mut self.0, context.allocator, context.program, scoping, ())
    }
}

const PROBE_SOURCE: &str = "const v: number = before;\nexport const e = <b>{v}</b>;\n";

#[test]
fn before_passes_see_types_and_jsx() {
    let mut pass = BeforePass::default();
    let ret = compile(Path::new("m.tsx"), PROBE_SOURCE, &options(), &mut pass);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(pass.0.saw_types && pass.0.saw_jsx, "the pass ran after the transforms");
    assert!(ret.code.contains("= after"), "{}", ret.code);
}

#[test]
fn after_passes_see_compiled_code() {
    let mut pass = AfterPass::default();
    let ret = compile(Path::new("m.tsx"), PROBE_SOURCE, &options(), &mut pass);
    assert!(ret.errors.is_empty(), "{:?}", ret.errors);
    assert!(!pass.0.saw_types && !pass.0.saw_jsx, "the pass ran before the transforms");
    assert!(ret.code.contains("= after"), "{}", ret.code);
}
