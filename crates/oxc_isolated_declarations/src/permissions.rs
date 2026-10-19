use oxc_ast::ast::*;
use oxc_ast_visit::{VisitMut, walk_mut};

/// TightScript: declarations spell permission words in full (`rw` becomes `readwrite`), as tsc does.
/// The permission after `deep` on a type parameter (`<deep rw T>`) is an identifier and stays as written.
pub struct LongPermissions;

fn lengthen(permission: &mut Option<TSTypeOperatorOperator>) {
    if let Some(permission) = permission {
        *permission = permission.to_long_permission();
    }
}

impl<'a> VisitMut<'a> for LongPermissions {
    fn visit_ts_type_operator(&mut self, it: &mut TSTypeOperator<'a>) {
        it.operator = it.operator.to_long_permission();
        walk_mut::walk_ts_type_operator(self, it);
    }

    fn visit_property_definition(&mut self, it: &mut PropertyDefinition<'a>) {
        lengthen(&mut it.permission);
        walk_mut::walk_property_definition(self, it);
    }

    fn visit_accessor_property(&mut self, it: &mut AccessorProperty<'a>) {
        lengthen(&mut it.permission);
        walk_mut::walk_accessor_property(self, it);
    }

    fn visit_ts_property_signature(&mut self, it: &mut TSPropertySignature<'a>) {
        lengthen(&mut it.permission);
        walk_mut::walk_ts_property_signature(self, it);
    }

    fn visit_ts_index_signature(&mut self, it: &mut TSIndexSignature<'a>) {
        lengthen(&mut it.permission);
        walk_mut::walk_ts_index_signature(self, it);
    }

    fn visit_ts_named_tuple_member(&mut self, it: &mut TSNamedTupleMember<'a>) {
        lengthen(&mut it.permission);
        walk_mut::walk_ts_named_tuple_member(self, it);
    }

    fn visit_ts_mapped_type(&mut self, it: &mut TSMappedType<'a>) {
        lengthen(&mut it.permission);
        walk_mut::walk_ts_mapped_type(self, it);
    }
}
