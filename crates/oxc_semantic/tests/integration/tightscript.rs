use oxc_semantic::SymbolFlags;

use crate::util::SemanticTester;

#[test]
fn distribute_member_is_bound_in_the_body() {
    SemanticTester::ts("type C<A> = distribute A Member as Member & { baz: number };")
        .has_some_symbol("Member")
        .contains_flags(SymbolFlags::TypeParameter)
        .has_number_of_references(1)
        .test();

    SemanticTester::ts("type C<A> = distribute A M as M[];")
        .has_some_symbol("A")
        .has_number_of_references(1)
        .test();
}

#[test]
fn type_from_declares_a_type() {
    SemanticTester::ts("export type R from f(1);\nlet r: R;")
        .has_root_symbol("R")
        .contains_flags(SymbolFlags::TypeAlias)
        .has_number_of_references(1)
        .test();
}

#[test]
fn provided_alias_is_declared_in_the_function() {
    SemanticTester::ts(
        "function p3<T>(v: T) { provide type B = Box<T>\n const b: B = { value: v }; return b; }",
    )
    .has_some_symbol("B")
    .contains_flags(SymbolFlags::TypeAlias)
    .has_number_of_references(1)
    .test();
}
