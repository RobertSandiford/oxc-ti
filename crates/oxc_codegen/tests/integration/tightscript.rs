use oxc_codegen::CodegenOptions;
use oxc_parser::ParseOptions;
use oxc_span::SourceType;

use crate::tester::{test_options_with_source_type, test_ts, test_with_parse_options};

#[track_caller]
fn test_ts_same(source_text: &str) {
    test_ts(source_text, source_text);
}

#[test]
fn permission_type_operators() {
    test_ts_same("type P1 = read Foo;\n");
    test_ts_same("type P2 = rw Foo;\n");
    test_ts_same("type P3 = readwritedelete string[];\n");
    test_ts_same("type P4 = lazy -write Foo;\n");
    test_ts_same("type P5 = +d Foo;\n");
    test_ts_same("type P6 = none A | all B;\n");
    test_ts_same("type P7 = =rw Foo;\n");
    test_ts_same("type P8 = delete Foo;\n");
    test_ts_same("type P9 = r rw Foo;\n");
    test_ts("type P10 = rw { a: string };", "type P10 = rw {\n\ta: string;\n};\n");
}

#[test]
fn ownership_type_operators() {
    test_ts_same("type O1 = owned Foo;\n");
    test_ts_same("type O2 = exact Foo;\n");
    test_ts_same("type O3 = -owned O1;\n");
    test_ts_same("type O4 = -exact O2;\n");
    test_ts_same("type O5 = deep exact Foo;\n");
    test_ts_same("type O6 = deep -exact Foo;\n");
    test_ts(
        "type O7 = exact (a: string) => string;\n",
        "type O7 = exact ((a: string) => string);\n",
    );
    test_ts_same("declare function keeps(x: transient Foo): void;\n");
    test_ts_same("const as1 = v as deep rw;\n");
    test_ts_same("function lazyDeep<lazy deep rwd T>(x: T) {}\n");
    test_ts_same("const as2 = v as exact Foo;\n");
}

#[test]
fn tuple_operand_needs_space() {
    test_ts_same("type T = r [string];\n");
    test_options_with_source_type(
        "type T = r [string];",
        "type T=r [string];",
        SourceType::ts(),
        CodegenOptions::minify(),
    );
}

#[test]
fn operator_words_stay_type_names() {
    test_ts_same("type UsesR = r[];\n");
    test_ts_same("let x: read;\n");
    test_ts_same("type X = read | T;\n");
    test_ts_same("type X = read & T;\n");
    test_ts_same("type X = read<T>;\n");
    test_ts_same("type X = owned;\n");
    test_ts_same("type X = exact[];\n");
    test_ts_same("type X = deep;\n");
    test_ts_same("type X = lazy;\n");
    test_ts_same("type X = -1;\n");
    // `{` on a new line isn't an operand, so this is a type `r`, then a block.
    test_ts("type X = r\n{ a: string };", "type X = r;\n{\n\ta: string;\n}\n;\n");
}

#[test]
fn member_permission_modifiers() {
    test_ts(
        "type M1 = { read a: string; rw b: number; readonly c: boolean };",
        "type M1 = {\n\tread a: string;\n\trw b: number;\n\treadonly c: boolean;\n};\n",
    );
    test_ts(
        "interface M2 { write a: string; r b: number }",
        "interface M2 {\n\twrite a: string;\n\tr b: number;\n}\n",
    );
    test_ts_same("type M3 = [read a: string, delete b?: number];\n");
    test_ts_same("type M3b = [rw ...rest: string[]];\n");
    test_ts(
        "class M4 { read x = 1; private rw y: string = \"y\"; static r z = 3; declare w q: number; accessor rw t = 1; }",
        "class M4 {\n\tread x = 1;\n\tprivate rw y: string = \"y\";\n\tstatic r z = 3;\n\tdeclare w q: number;\n\taccessor rw t = 1;\n}\n",
    );
    test_ts("type I = { read [key: string]: T };", "type I = {\n\tread [key: string]: T;\n};\n");
    test_options_with_source_type(
        "class C { rw x = 1; static rw [k: string]: number }",
        "class C{rw x=1;static rw [k:string]:number}",
        SourceType::ts(),
        CodegenOptions::minify(),
    );
}

#[test]
fn member_permission_words_stay_names() {
    test_ts(
        "type T = { read: string; rw(): void; r?: number };",
        "type T = {\n\tread: string;\n\trw(): void;\n\tr?: number;\n};\n",
    );
    test_ts("class C { rw = 3; r() {} }", "class C {\n\trw = 3;\n\tr() {}\n}\n");
    test_ts_same("type T = [r: string];\n");
}

#[test]
fn mapped_type_permission_modifiers() {
    test_ts_same("type MP1<T> = { read [K in keyof T]: T[K] };\n");
    test_ts_same("type MP2<T> = { -write [K in keyof T]: T[K] };\n");
    test_ts_same("type MP3<T> = { readonly +rw [K in keyof T]?: T[K] };\n");
    test_ts_same("type MP4<T> = { +readonly [K in keyof T]: T[K] };\n");
    test_options_with_source_type(
        "type MP<T> = { readonly rw [K in keyof T]: T[K] };",
        "type MP<T>={readonly rw[K in keyof T]:T[K]};",
        SourceType::ts(),
        CodegenOptions::minify(),
    );
}

#[test]
fn type_parameter_forms() {
    test_ts_same("function tp1<owned T>(x: T): T {\n\treturn x;\n}\n");
    test_ts_same("function tp2<exact? T>(x: T): T {\n\treturn x;\n}\n");
    test_ts_same("function tp3<owned exact? T>(x: T): T {\n\treturn x;\n}\n");
    test_ts("function tp4<exact const T>(x: T) {}", "function tp4<const exact T>(x: T) {}\n");
    test_ts_same("function tp4b<const exact T>(x: T) {}\n");
    test_ts_same("function tp5<deep rw T>(x: T) {}\n");
    test_ts_same("function tp6<T as exact extends {}>(x: T) {}\n");
    test_ts_same("function tp7<T as deep exact extends {}>(x: T) {}\n");
    test_ts("const arrow1 = <owned T,>(x: T) => x;", "const arrow1 = <owned T>(x: T) => x;\n");
    test_ts_same("const arrow2 = <deep rw T>(x: T) => x;\n");
    test_ts_same("class C<exact T> {}\n");
    test_ts_same("interface I<owned T> {}\n");
    test_ts_same("type A<exact T> = T;\n");
}

#[test]
fn type_parameter_hint_words_stay_names() {
    test_ts_same("function f<exact>() {}\n");
    test_ts("function f<owned, exact>() {}", "function f<\n\towned,\n\texact\n>() {}\n");
    // `owned` is a hint here and `exact` the name.
    test_ts_same("function f<owned exact>() {}\n");
    test_ts_same("function f<deep>() {}\n");
    test_ts_same("function f<T extends exact>() {}\n");
}

#[test]
fn tsx_deep_type_parameter_is_jsx_without_comma() {
    let tsx = |source: &str, expected: &str| {
        test_options_with_source_type(
            source,
            expected,
            SourceType::tsx(),
            CodegenOptions::default(),
        );
    };
    tsx("const a = <deep rw T,>(x: T) => x;", "const a = <deep rw T,>(x: T) => x;\n");
    tsx("const a = <const deep rw T,>(x: T) => x;", "const a = <const deep rw T,>(x: T) => x;\n");
    tsx(
        "const a = <deep rw T extends U>(x: T) => x;",
        "const a = <deep rw T extends U,>(x: T) => x;\n",
    );
}

#[test]
fn heritage_minus_exact() {
    test_ts(
        "interface Child extends -exact Base { b: string }",
        "interface Child extends -exact Base {\n\tb: string;\n}\n",
    );
    test_ts("interface C extends A, -exact B<T> {}", "interface C extends A, -exact B<T> {}\n");
    test_ts(
        "class D implements -exact A { a = 1 }",
        "class D implements -exact A {\n\ta = 1;\n}\n",
    );
}

#[test]
fn soft_type_guards() {
    test_ts(
        "function isStr(x: unknown): x is? string { return typeof x === \"string\"; }",
        "function isStr(x: unknown): x is? string {\n\treturn typeof x === \"string\";\n}\n",
    );
    test_ts(
        "class G { isMe(): this is? G { return true; } }",
        "class G {\n\tisMe(): this is? G {\n\t\treturn true;\n\t}\n}\n",
    );
    test_ts_same("function assertStr(x: unknown): asserts x is? string {}\n");
    test_ts_same("function isStr(x: unknown): x is string {}\n");
}

#[test]
fn distribute_types() {
    test_ts_same("type D1<U> = distribute U as U[];\n");
    test_ts(
        "type D2<U> = distribute U M as { m: M };",
        "type D2<U> = distribute U M as {\n\tm: M;\n};\n",
    );
    test_ts_same("type D3<U> = string | distribute U as U[];\n");
    test_ts_same("type F<P> = distribute S as M<P, S> extends true ? P : never;\n");
    test_ts_same("type X = distribute;\n");
    test_ts_same("type X = distribute[];\n");
    test_ts("function f(): distribute {}", "function f(): distribute {}\n");
}

#[test]
fn open_ended_types_keep_parentheses_where_more_follows() {
    let options = ParseOptions { preserve_parens: false, ..ParseOptions::default() };
    let cases = [
        ("type A = (distribute T as T)[];", "type A = (distribute T as T)[];\n"),
        ("type A = (distribute T as T) | X;", "type A = (distribute T as T) | X;\n"),
        ("type A = X | distribute T as T;", "type A = X | distribute T as T;\n"),
        ("type A = rw (distribute T as T);", "type A = rw (distribute T as T);\n"),
        ("declare function f(): (type T) | X;", "declare function f(): (type T) | X;\n"),
    ];
    for (source, expected) in cases {
        test_with_parse_options(source, expected, options);
    }
}

#[test]
fn function_typed_by_annotation() {
    test_ts(
        "export function f1: F (a) { return a }",
        "export function f1: F (a) {\n\treturn a;\n}\n",
    );
    test_ts(
        "export function f3: (GF) <T>(a: T) { return a }",
        "export function f3: (GF) <T>(a: T) {\n\treturn a;\n}\n",
    );
    test_ts(
        "export async function f4: ((a: string) => Promise<string>) (a) { return a }",
        "export async function f4: ((a: string) => Promise<string>) (a) {\n\treturn a;\n}\n",
    );
    test_ts_same("export declare function f6: F;\n");
}

#[test]
fn function_type_annotation_parenthesized_before_own_type_parameters() {
    let source = "export function f3: (GF) <T>(a: T) { return a }";
    let options = ParseOptions { preserve_parens: false, ..ParseOptions::default() };
    // This harness parses as .tsx, where codegen writes `<T,>`.
    test_with_parse_options(
        source,
        "export function f3: (GF) <T,>(a: T) {\n\treturn a;\n}\n",
        options,
    );
}

#[test]
fn type_returns() {
    test_ts_same("export function f<G>(v: G): type Box<G> {}\n");
    test_ts(
        "export function hy<G>(v: G): boolean and type Box<G> { return true; }",
        "export function hy<G>(v: G): boolean and type Box<G> {\n\treturn true;\n}\n",
    );
    test_ts_same("export const arrow = <G>(v: G): boolean and type Box<G> => v !== undefined;\n");
    test_ts(
        "class C { static s(): type number {} }",
        "class C {\n\tstatic s(): type number {}\n}\n",
    );
    test_ts_same("export type TF = <G>(v: G) => type Box<G>;\n");
    test_ts_same("declare function takes<T>(fn: () => void and type T): type T;\n");
    test_ts_same("type X = type[];\n");
}

#[test]
fn provide_statements() {
    test_ts(
        "function p1(): number {\n\tprovide Box<string>\n\treturn 1;\n}",
        "function p1(): number {\n\tprovide Box<string>;\n\treturn 1;\n}\n",
    );
    test_ts_same("function p2<T>(v: T) {\n\tprovide from\n\tg(v);\n}\n");
    test_ts(
        "function p3<T>(v: T) {\n\tprovide type B = Box<T>\n\tconst b: B = { value: v };\n\treturn b;\n}",
        "function p3<T>(v: T) {\n\tprovide type B = Box<T>;\n\tconst b: B = { value: v };\n\treturn b;\n}\n",
    );
    test_ts_same(
        "function p5<T>(v: T) {\n\tprovide type G from\n\tconst n = gv(v);\n\treturn n;\n}\n",
    );
    test_ts_same("provide(x);\n");
    test_ts_same("provide = 9;\n");
    test_ts("provide\nx;", "provide;\nx;\n");
}

#[test]
fn type_from_declarations() {
    test_ts("export type R from f(1);", "export type R from\nf(1);\n");
    test_ts("type RN from\nf<string>('s');", "type RN from\nf<string>(\"s\");\n");
    test_ts(
        "export type RV from\nexport const rv = hy(1);",
        "export type RV from\nexport const rv = hy(1);\n",
    );
    test_ts("type RS from\n(f)(v)", "type RS from\nf(v);\n");
    test_options_with_source_type(
        "export type R from f(1);",
        "export type R from f(1);",
        SourceType::ts(),
        CodegenOptions::minify(),
    );
}

#[test]
fn override_casts() {
    test_ts_same("const o1 = u override string;\n");
    test_ts_same("const o2 = s override! number;\n");
    test_ts_same("const o3 = <override string>u;\n");
    test_ts_same("const o4 = <override! number>s;\n");
    test_ts_same("const o5 = (s override! number) + 1;\n");
    test_ts("const o6 = s override !number;", "const o6 = s override! number;\n");
}

#[test]
fn type_literal_spreads() {
    test_ts(
        "type S = { ...A, b: number, ...B };",
        "type S = {\n\t...A;\n\tb: number;\n\t...B;\n};\n",
    );
    test_ts("type S = { ...A | undefined };", "type S = {\n\t...A | undefined;\n};\n");
}

#[test]
fn brace_after_colon_return_type() {
    test_ts("function retR(): r { return \"x\"; }", "function retR(): r {\n\treturn \"x\";\n}\n");
    test_ts("function f(): r {}", "function f(): r {}\n");
    test_ts("function f(): r { a: string } {}", "function f(): r {\n\ta: string;\n} {}\n");
    test_ts("function f(): r { [K in T]: K } {}", "function f(): r { [K in T]: K } {}\n");
    test_ts_same("type F = () => r {};\n");
}
