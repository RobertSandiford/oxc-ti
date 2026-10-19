use oxc_ast::ast::*;
use oxc_span::GetSpan;

use super::NeedsParentheses;
use crate::{
    ast_nodes::{AstNode, AstNodes},
    formatter::JsFormatter,
};

/// Looks through single-member `TSUnionType` and `TSIntersectionType` to find the effective parent.
///
/// In Prettier's AST (Babel), single-member unions/intersections (e.g., from leading `|` in
/// `(| number)[]`) don't exist — the parser unwraps them. In oxc's AST, they do exist, so inner
/// types need to "see through" them when checking `needs_parentheses` to get the correct parent
/// context.
fn effective_parent<'a>(parent: &'a AstNodes<'a>) -> &'a AstNodes<'a> {
    match parent {
        AstNodes::TSUnionType(union) if union.types.len() <= 1 => effective_parent(union.parent()),
        AstNodes::TSIntersectionType(intersection) if intersection.types.len() <= 1 => {
            effective_parent(intersection.parent())
        }
        other => other,
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSType<'_>> {
    fn needs_parentheses(&self, f: &JsFormatter<'_, '_>) -> bool {
        match self.as_ast_nodes() {
            AstNodes::TSFunctionType(it) => it.needs_parentheses(f),
            AstNodes::TSInferType(it) => it.needs_parentheses(f),
            AstNodes::TSConstructorType(it) => it.needs_parentheses(f),
            AstNodes::TSUnionType(it) => it.needs_parentheses(f),
            AstNodes::TSIntersectionType(it) => it.needs_parentheses(f),
            AstNodes::TSConditionalType(it) => it.needs_parentheses(f),
            AstNodes::TSTypeOperator(it) => it.needs_parentheses(f),
            AstNodes::TSTypeQuery(it) => it.needs_parentheses(f),
            AstNodes::TSDistributeType(it) => it.needs_parentheses(f),
            AstNodes::TSTypeReturn(it) => it.needs_parentheses(f),
            _ => {
                // TODO: incomplete
                false
            }
        }
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSFunctionType<'_>> {
    #[inline]
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        let parent = effective_parent(self.parent());

        // TSFunctionType needs parens when used as an arrow function's return type
        // to resolve syntactic ambiguity: `(): () => void => ...` is ambiguous without parens.
        // TSConstructorType does NOT need this — the `new` keyword is unambiguous.
        //
        // https://github.com/prettier/prettier/blob/812a4d0071270f61a7aa549d625b618be7e09d71/src/language-js/parentheses/needs-parentheses.js#L514-L525
        if let AstNodes::TSTypeAnnotation(type_annotation) = parent
            && matches!(type_annotation.parent(), AstNodes::ArrowFunctionExpression(_))
        {
            return true;
        }

        function_like_type_needs_parentheses(self.span(), parent, Some(&self.return_type))
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSInferType<'_>> {
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        let parent = effective_parent(self.parent());
        match parent {
            AstNodes::TSIntersectionType(_) | AstNodes::TSUnionType(_) => true,
            AstNodes::TSRestType(_) => false,
            _ => operator_type_or_higher_needs_parens(self.span, parent),
        }
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSConstructorType<'_>> {
    #[inline]
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        function_like_type_needs_parentheses(
            self.span(),
            effective_parent(self.parent()),
            Some(&self.return_type),
        )
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSUnionType<'_>> {
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        // Single-member unions are transparent (formatted as just the member).
        // In Prettier/Babel, these don't exist in the AST.
        if self.types.len() <= 1 {
            return false;
        }
        match effective_parent(self.parent()) {
            AstNodes::TSUnionType(union) => union.types.len() > 1,
            AstNodes::TSIntersectionType(intersection) => intersection.types.len() > 1,
            parent => operator_type_or_higher_needs_parens(self.span(), parent),
        }
    }
}

/// Returns `true` if a function-like TS type (TSFunctionType, TSConstructorType)
/// needs parentheses based on their parent context.
///
/// Note: The arrow-function-return-type case is handled separately by each caller,
/// because only `TSFunctionType` needs parens there (syntactic ambiguity with `=>`),
/// while `TSConstructorType` does not (the `new` keyword is unambiguous).
///
/// Ported from Biome's function_like_type_needs_parentheses
fn function_like_type_needs_parentheses<'a>(
    span: Span,
    parent: &'a AstNodes<'a>,
    return_type: Option<&'a TSTypeAnnotation<'a>>,
) -> bool {
    match parent {
        // In conditional types
        AstNodes::TSConditionalType(conditional) => {
            let is_check_type = conditional.check_type().span() == span;
            if is_check_type {
                return true;
            }

            let is_extends_type = conditional.extends_type().span() == span;
            if is_extends_type {
                // Need parentheses if return type is TSInferType with constraint
                // or TSTypePredicate with type annotation
                if let Some(return_type) = return_type {
                    match &return_type.type_annotation {
                        TSType::TSInferType(infer_type) => {
                            return infer_type.type_parameter.constraint.is_some();
                        }
                        TSType::TSTypePredicate(predicate) => {
                            return predicate.type_annotation.is_some();
                        }
                        _ => {}
                    }
                }
            }
            false
        }
        AstNodes::TSUnionType(union) => union.types.len() > 1,
        AstNodes::TSIntersectionType(intersection) => intersection.types.len() > 1,
        _ => operator_type_or_higher_needs_parens(span, parent),
    }
}

/// Returns `true` if a TS primary type needs parentheses
/// This is for types that have higher precedence operators as parents
fn operator_type_or_higher_needs_parens(span: Span, parent: &AstNodes) -> bool {
    match parent {
        // These parent types always require parentheses for their operands
        AstNodes::TSArrayType(_)
        | AstNodes::TSTypeOperator(_)
        | AstNodes::TSRestType(_)
        | AstNodes::TSOptionalType(_) => true,
        // Indexed access requires parens if this is the object type
        AstNodes::TSIndexedAccessType(indexed) => indexed.object_type.span() == span,
        _ => false,
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSIntersectionType<'_>> {
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        // Single-member intersections are transparent (formatted as just the member).
        if self.types.len() <= 1 {
            return false;
        }
        match effective_parent(self.parent()) {
            AstNodes::TSUnionType(union) => union.types.len() > 1,
            AstNodes::TSIntersectionType(intersection) => intersection.types.len() > 1,
            parent => operator_type_or_higher_needs_parens(self.span(), parent),
        }
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSConditionalType<'_>> {
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        let parent = effective_parent(self.parent());
        match parent {
            AstNodes::TSConditionalType(ty) => {
                ty.extends_type().span() == self.span() || ty.check_type().span() == self.span()
            }
            // A conditional type is not allowed bare in a type parameter's constraint position:
            // `<T extends (A extends B ? C : D)>` requires the parentheses.
            // The default position (`= ...`) does not.
            AstNodes::TSTypeParameter(param) => {
                param.constraint().is_some_and(|c| c.span() == self.span())
            }
            AstNodes::TSUnionType(union) => union.types.len() > 1,
            AstNodes::TSIntersectionType(intersection) => intersection.types.len() > 1,
            _ => operator_type_or_higher_needs_parens(self.span, parent),
        }
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSDistributeType<'_>> {
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        open_ended_type_needs_parens(self.span(), effective_parent(self.parent()))
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSTypeReturn<'_>> {
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        open_ended_type_needs_parens(self.span(), effective_parent(self.parent()))
    }
}

/// TightScript's `distribute U as X` and `type T` take the rest of the type as their operand,
/// so they need parentheses wherever more of the type follows them.
fn open_ended_type_needs_parens(span: Span, parent: &AstNodes) -> bool {
    match parent {
        AstNodes::TSConditionalType(ty) => {
            ty.extends_type().span() == span || ty.check_type().span() == span
        }
        AstNodes::TSUnionType(union) => union.types.last().is_some_and(|last| last.span() != span),
        AstNodes::TSIntersectionType(intersection) => {
            intersection.types.last().is_some_and(|last| last.span() != span)
        }
        _ => operator_type_or_higher_needs_parens(span, parent),
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSTypeOperator<'_>> {
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        let parent = effective_parent(self.parent());
        // TightScript operators chain without parens (`owned rw T`), and `deep` requires it (`deep exact T`).
        if let AstNodes::TSTypeOperator(operator) = parent
            && operator.operator.is_tightscript()
        {
            return false;
        }
        if operand_starts_with_empty_type_literal(self)
            && is_top_of_colon_return_type(self.span(), parent)
        {
            return true;
        }
        operator_type_or_higher_needs_parens(self.span(), parent)
    }
}

/// `rw {}`, `deep rw {}[]`: at the top of a function's `: T` return type, that `{}` would read as the
/// body. `owned`, `exact` and `transient` always take `{` as their operand.
fn operand_starts_with_empty_type_literal(operator: &TSTypeOperator<'_>) -> bool {
    fn starts_with_empty_type_literal(ty: &TSType<'_>) -> bool {
        match ty {
            TSType::TSTypeLiteral(literal) => literal.members.is_empty(),
            TSType::TSArrayType(array) => starts_with_empty_type_literal(&array.element_type),
            TSType::TSIndexedAccessType(access) => {
                starts_with_empty_type_literal(&access.object_type)
            }
            _ => false,
        }
    }
    if !operator.operator.is_tightscript() {
        return false;
    }
    match &operator.type_annotation {
        TSType::TSTypeOperatorType(inner) => operand_starts_with_empty_type_literal(inner),
        operand => {
            !matches!(
                operator.operator,
                TSTypeOperatorOperator::Owned
                    | TSTypeOperatorOperator::Exact
                    | TSTypeOperatorOperator::Transient
            ) && starts_with_empty_type_literal(operand)
        }
    }
}

/// Whether a type is parsed at the top level of a function's, method's or accessor's `: T` return
/// type, where a body may follow. Union and intersection members, operands of `keyof` and the like
/// and a conditional's checked type are parsed at that same level, and so is a type that ends the
/// return type: a predicate's type, `type T`, a function type's `=> T`.
fn is_top_of_colon_return_type(span: Span, parent: &AstNodes<'_>) -> bool {
    match parent {
        AstNodes::TSUnionType(union) => is_top_of_colon_return_type(union.span, union.parent()),
        AstNodes::TSIntersectionType(intersection) => {
            is_top_of_colon_return_type(intersection.span, intersection.parent())
        }
        AstNodes::TSTypeOperator(operator) => {
            is_top_of_colon_return_type(operator.span, operator.parent())
        }
        AstNodes::TSConditionalType(conditional) if conditional.check_type.span() == span => {
            is_top_of_colon_return_type(conditional.span, conditional.parent())
        }
        AstNodes::TSTypeAnnotation(annotation) => match annotation.parent() {
            AstNodes::Function(_) => true,
            AstNodes::TSTypePredicate(predicate) => {
                is_top_of_colon_return_type(predicate.span, predicate.parent())
            }
            AstNodes::TSFunctionType(function) => {
                is_top_of_colon_return_type(function.span, function.parent())
            }
            _ => false,
        },
        AstNodes::TSTypeReturn(type_return) => {
            is_top_of_colon_return_type(type_return.span, type_return.parent())
        }
        AstNodes::TSDistributeType(distribute) if distribute.body.span() == span => {
            is_top_of_colon_return_type(distribute.span, distribute.parent())
        }
        _ => false,
    }
}

impl NeedsParentheses<'_> for AstNode<'_, TSTypeQuery<'_>> {
    fn needs_parentheses(&self, _f: &JsFormatter<'_, '_>) -> bool {
        match effective_parent(self.parent()) {
            AstNodes::TSArrayType(_) => true,
            // Typeof operators are parenthesized when used as an object type in an indexed access
            // to avoid ambiguity of precedence, as it's higher than the JS equivalent:
            // ```typescript
            // const array = [1, 2, 3]
            // type T = typeof array[0]; // => number
            // type T2 = (typeof array)[0]; // => number
            // const J1 = typeof array[0]; // => 'number'
            // const J2 = (typeof array)[0]; // => 'o', because `typeof array` is 'object'
            // ```
            AstNodes::TSIndexedAccessType(indexed) => {
                // The typeof operator only needs parens if it's the object of the indexed access.
                // If it's the index_type, then the braces already act as the visual precedence.
                indexed.object_type().span() == self.span()
            }
            _ => false,
        }
    }
}
