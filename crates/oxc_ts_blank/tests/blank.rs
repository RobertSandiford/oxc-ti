use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ts_blank::blank;

fn minified(js: &str, source_type: SourceType) -> String {
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, js, source_type).parse();
    assert!(ret.diagnostics.is_empty(), "not valid JS: {:?}\n{js}", ret.diagnostics);
    Codegen::new().with_options(CodegenOptions::minify()).build(&ret.program).code
}

/// Each output line has its source line's UTF-16 length, and every character is kept,
/// blanked to a space, or one of the `;` and `)` the blanker places on purpose.
#[track_caller]
fn assert_same_layout(ts: &str, js: &str) {
    let ts_lines: Vec<_> = ts.split('\n').collect();
    let js_lines: Vec<_> = js.split('\n').collect();
    assert_eq!(ts_lines.len(), js_lines.len(), "line count changed:\n{js}");
    for (ts_line, js_line) in ts_lines.iter().zip(&js_lines) {
        let ts_units: Vec<u16> = ts_line.encode_utf16().collect();
        let js_units: Vec<u16> = js_line.encode_utf16().collect();
        assert_eq!(
            ts_units.len(),
            js_units.len(),
            "line length changed: {ts_line:?} -> {js_line:?}"
        );
        for (t, j) in ts_units.iter().zip(&js_units) {
            assert!(
                t == j || [u16::from(b' '), u16::from(b';'), u16::from(b')')].contains(j),
                "character changed: {ts_line:?} -> {js_line:?}"
            );
        }
    }
}

#[track_caller]
fn test_with(ts: &str, source_type: SourceType, expected_js: &str) {
    let ret = blank(ts, source_type);
    assert!(ret.errors.is_empty(), "{ts:?}: {:?}", ret.errors);
    assert_same_layout(ts, &ret.code);
    let js_type = SourceType::mjs().with_jsx(source_type.is_jsx());
    assert_eq!(
        minified(&ret.code, js_type),
        minified(expected_js, js_type),
        "\nsource:  {ts:?}\nblanked: {:?}",
        ret.code
    );
}

#[track_caller]
fn test(ts: &str, expected_js: &str) {
    test_with(ts, SourceType::ts(), expected_js);
}

#[track_caller]
fn test_exact(ts: &str, expected: &str) {
    let ret = blank(ts, SourceType::ts());
    assert!(ret.errors.is_empty(), "{ts:?}: {:?}", ret.errors);
    assert_eq!(ret.code, expected);
}

#[track_caller]
fn test_error(ts: &str, message: &str) {
    let ret = blank(ts, SourceType::ts());
    assert!(
        ret.errors.iter().any(|e| e.to_string().contains(message)),
        "{ts:?}: expected an error containing {message:?}, got {:?}",
        ret.errors
    );
}

#[test]
fn keeps_positions() {
    test_exact("let x: number = 1;", "let x         = 1;");
    test_exact("f<T>(a as any)!;", "f   (a       ) ;");
}

#[test]
fn annotations() {
    test("let x: number = 1, y!: string;", "let x = 1, y;");
    test("function f(a: string, b?: number, ...c: any[]): void {}", "function f(a, b, ...c) {}");
    test(
        "const g = function (a?: number): string { return '' };",
        "const g = function (a) { return '' };",
    );
    test("const { a, b }: { a: number; b: number } = o;", "const { a, b } = o;");
    test("for (const x of xs as number[]) {}", "for (const x of xs) {}");
}

#[test]
fn type_parameters_and_arguments() {
    test("function id<T>(x: T): T { return x }", "function id(x) { return x }");
    test("id<number>(1); new Map<string, number>(); tag<T>`x`;", "id(1); new Map(); tag`x`;");
    test("class C<T> extends B<T> {}", "class C extends B {}");
    test("const h = f<string>;", "const h = f;");
    test("const a = <T,>(x: T) => x;", "const a = (x) => x;");
}

#[test]
fn expressions() {
    test("const a = b as any, c = d satisfies X, e = <T>f;", "const a = b, c = d, e = f;");
    test("g!.h; i!();", "g.h; i();");
    test("const j = k as const;", "const j = k;");
}

#[test]
fn type_only_declarations() {
    test(
        "type A = 1; interface I {} declare const z: number; declare function df(): void; declare class DC {}",
        ";;;;;",
    );
    test(
        "declare module 'm' {} declare global {} declare namespace N {} namespace T { export type X = 1 } declare enum E { A }",
        ";;;;;",
    );
    test("function o(a: number): void; function o(a) {}", "; function o(a) {}");
    test("export type A = 1; export interface I {} export declare const z: number;", ";;;");
    test("export default interface I {}", ";");
}

#[test]
fn imports_and_exports() {
    test(
        "import type A from 'a'; import { type B, c } from 'b'; import type * as NS from 'y';",
        "; import { c } from 'b'; ;",
    );
    test("export type { D }; export { type E, f }; export type * from 'x';", "; export { f }; ;");
    test("import { c, type B } from 'b';", "import { c } from 'b';");
    test("import type X = require('x'); export as namespace Lib;", ";;");
}

#[test]
fn classes() {
    test(
        "abstract class A implements I, J { private x: number = 1; public readonly y?: string; protected abstract z: number; declare w: string; t!: number; static s: number; [k: string]: any }",
        "class A { x = 1; y; ; ; t; static s; ; }",
    );
    test(
        "class M { public m?(): void {} abstract am(): void; o(): void; o(a?: number) {} override toString(): string { return '' } }",
        "class M { m() {} ; ; o(a) {} toString() { return '' } }",
    );
    test(
        "class G { get v(): number { return 1 } set v(x: number) {} accessor q: number = 1 }",
        "class G { get v() { return 1 } set v(x) {} accessor q = 1 }",
    );
    test("const K = class<T> implements I {};", "const K = class {};");
}

#[test]
fn this_parameter() {
    test("function f(this: Window, a: number) {}", "function f(a) {}");
    test("function g(this: Window) {}", "function g() {}");
}

#[test]
fn statement_boundaries() {
    test("let a = b\ntype T = 1\n(c)()", "let a = b\n;\n(c)()");
    test("x = y as T\n[1].forEach(f)", "x = y;\n[1].forEach(f)");
    // TypeScript reads this as the call `y!(z)`, so no `;` belongs here.
    test("x = y!\n(z)", "x = y(z)");
    test("class F { a: number\n[k] = 1 }", "class F { a;\n[k] = 1 }");
    test("class A { x = 1\n  read [\"k\"] = 2 }", "class A { x = 1;\n [\"k\"] = 2 }");
    test("class A { x = 1\n  private [\"k\"] = 2 }", "class A { x = 1;\n [\"k\"] = 2 }");
    test("class A { x = 1\n  public *g() {} }", "class A { x = 1;\n *g() {} }");
    test("class A { x = 1\n  static [\"k\"] = 2 }", "class A { x = 1\n static [\"k\"] = 2 }");
}

#[test]
fn arrow_return_type_across_lines() {
    test("const f = (a): {\n  x: number\n} => a;", "const f = (a) => a;");
    test_exact("const f = (a): {\n  x: 1\n} => a;", "const f = (a    \n      \n) => a;");
}

#[test]
fn tightscript_types_and_members() {
    test("let x: owned rw { a: string } = { a: '' };", "let x = { a: '' };");
    test("const as1 = v as deep rw, as2 = v as exact { a: string };", "const as1 = v, as2 = v;");
    test(
        "class M4 { read x = 1; private rw y: string = 'y'; static r z = 3; declare w q: number; rw [k] = 2; accessor rw p = 1 }",
        "class M4 { x = 1; y = 'y'; static z = 3; ; [k] = 2; accessor p = 1 }",
    );
    test("class D implements -exact A { a = 1 }", "class D { a = 1 }");
    test(
        "function isStr(x: unknown): x is? string { return true }",
        "function isStr(x) { return true }",
    );
    test("type D3<U> = string | distribute U as U[]; let y: D3<number>;", "; let y;");
    test("function g<const exact T, deep rw S>(v: T, s: S) {}", "function g(v, s) {}");
}

#[test]
fn tightscript_function_types() {
    test("export function f1: F (a) { return a }", "export function f1(a) { return a }");
    test("export function f3: (GF) <T>(a: T) { return a }", "export function f3(a) { return a }");
    test("export declare function f6: F;", ";");
}

#[test]
fn tightscript_type_returns() {
    test("export function f<G>(v: G): type Box<G> {}", "export function f(v) {}");
    test(
        "export const arrow = <G,>(v: G): boolean and type Box<G> => v !== undefined;",
        "export const arrow = (v) => v !== undefined;",
    );
}

#[test]
fn tightscript_provide_and_type_from() {
    test(
        "function p1(): number { provide Box<string>\n return 1; }",
        "function p1() { ;\n return 1; }",
    );
    test("function p2<T>(v: T) { provide from\n g(v); }", "function p2(v) { ;\n g(v); }");
    test(
        "function p3<T>(v: T) { provide type B = Box<T>\n const b: B = { value: v }; return b; }",
        "function p3(v) { ;\n const b = { value: v }; return b; }",
    );
    test("export type R from f(1);", "; f(1);");
    test("export type RV from\nexport const rv = hy(1);", ";\nexport const rv = hy(1);");
    test("a()\ntype RS from\n(f)(v)", "a()\n;\n(f)(v)");
}

#[test]
fn tsx() {
    test_with(
        "const a = <T,>(x: T) => <div>{x as any}</div>; const b = <Comp<string> p={1 as number} />;",
        SourceType::tsx(),
        "const a = (x) => <div>{x}</div>; const b = <Comp p={1} />;",
    );
}

#[test]
fn utf16_columns() {
    test("let s: 'é𝄞' = 'é𝄞';", "let s = 'é𝄞';");
}

#[test]
fn unsupported_syntax() {
    test_error("enum E { A }", "enum");
    test_error("const enum E { A }", "enum");
    test_error("namespace N { export const a = 1 }", "namespace");
    test_error("class P { constructor(private p: number) {} }", "parameter propert");
    test_error("import x = require('x');", "import =");
    test_error("export = x;", "export =");
}

#[test]
fn reports_parse_errors() {
    test_error("let x: = 1;", "Unexpected token");
}
