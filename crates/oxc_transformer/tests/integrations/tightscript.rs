use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_transformer::TransformOptions;

use crate::{codegen, test_with_source_type};

#[track_caller]
fn test_emit(source_text: &str, expected: &str) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source_text, SourceType::ts()).parse();
    assert!(parsed.diagnostics.is_empty(), "{source_text:?}: {:?}", parsed.diagnostics);
    let result =
        test_with_source_type(source_text, SourceType::ts(), &TransformOptions::default()).unwrap();
    assert_eq!(result, codegen(expected, SourceType::mjs()), "\nfor source: {source_text:?}");
}

#[test]
fn type_operators_are_erased() {
    test_emit("const as1 = v as deep rw;", "const as1 = v;");
    test_emit("const as2 = v as exact { a: string };", "const as2 = v;");
    test_emit("let x: owned rw { a: string } = { a: '' };", "let x = { a: '' };");
    test_emit("function f(): r { return 1; }", "function f() { return 1; }");
}

#[test]
fn class_fields_keep_value_and_lose_permission() {
    test_emit(
        "class M4 { read x = 1; private rw y: string = 'y'; static r z = 3; declare w q: number; }",
        "class M4 { x = 1; y = 'y'; static z = 3; }",
    );
}

#[test]
fn heritage_and_guards_are_erased() {
    test_emit("class D implements -exact A { a = 1 }", "class D { a = 1; }");
    test_emit(
        "function isStr(x: unknown): x is? string { return true; }",
        "function isStr(x) { return true; }",
    );
}

#[test]
fn function_type_annotation_is_erased() {
    test_emit("export function f1: F (a) { return a }", "export function f1(a) { return a; }");
    test_emit(
        "export function f3: (GF) <T>(a: T) { return a }",
        "export function f3(a) { return a; }",
    );
    test_emit("export declare function f6: F;", "export {};");
}

#[test]
fn type_returns_are_erased() {
    test_emit("export function f<G>(v: G): type Box<G> {}", "export function f(v) {}");
    test_emit(
        "export const arrow = <G,>(v: G): boolean and type Box<G> => v !== undefined;",
        "export const arrow = (v) => v !== undefined;",
    );
}

#[test]
fn provide_statements_are_erased() {
    test_emit(
        "function p1(): number { provide Box<string>\n return 1; }",
        "function p1() { return 1; }",
    );
    test_emit("function p2<T>(v: T) { provide from\n g(v); }", "function p2(v) { g(v); }");
    test_emit(
        "function p3<T>(v: T) { provide type B = Box<T>\n const b: B = { value: v }; return b; }",
        "function p3(v) { const b = { value: v }; return b; }",
    );
}

#[test]
fn type_from_keeps_the_following_statement() {
    // The fork's tsc also adds `export {}` once every export is erased.
    test_emit("export type R from f(1);", "f(1);\nexport {};");
    test_emit("export type RN from\nf<string>('s');", "f('s');\nexport {};");
    test_emit("export type RV from\nexport const rv = hy(1);", "export const rv = hy(1);");
    test_emit("type RS from\n(f)(v)", "f(v);");
    test_emit("export type Tagged from tag`x${1}`;", "tag`x${1}`;\nexport {};");
}

#[test]
fn distribute_is_erased() {
    test_emit("type D3<U> = string | distribute U as U[]; let x: D3<number>;", "let x;");
}

#[test]
fn member_and_mapped_permissions_are_erased() {
    test_emit(
        "type M1 = { read a: string }; interface M2 { w b: number } type MP<T> = { -write [K in keyof T]: T[K] }; type M3 = [read a: string];",
        "",
    );
}
