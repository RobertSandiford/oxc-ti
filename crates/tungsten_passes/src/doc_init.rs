//! `BabelTransformDocInit`: a global `DocInit(fn, serverValue?)` call becomes
//! `_docInit(<id>, serverValue?)`, reading what `fn` returned when it ran in the document head.
//!
//! Babel inserts the runtime import at `Program` enter and rewrites the calls at `Program` exit,
//! after the presets have compiled the functions. Here the import is inserted in `before`, and the
//! callee renamed to it there too: a reference is what keeps the TypeScript transform from removing
//! an import nothing uses yet (Babel's survives by having no scope binding). The id, the check that
//! the function is self-contained, and the entry for the head script wait for `after`. JS writes the
//! head script from the entries.

use oxc_allocator::ArenaVec;
use oxc_ast::ast::{
    Argument, ArrayAssignmentTarget, AssignmentExpression, AssignmentTarget,
    AssignmentTargetMaybeDefault, AssignmentTargetProperty, CallExpression, Expression,
    ImportDeclarationSpecifier, ImportOrExportKind, ModuleExportName, NumberBase,
    ObjectAssignmentTarget, Program, Statement, StringLiteral, ThisExpression,
};
use oxc_ast::ast::{Function, IdentifierReference};
use oxc_ast_visit::{
    Visit,
    walk::{walk_assignment_expression, walk_call_expression, walk_function},
};
use oxc_codegen::Codegen;
use oxc_diagnostics::OxcDiagnostic;
use oxc_semantic::{ReferenceId, ScopeFlags, ScopeId, Scoping, SymbolFlags};
use oxc_span::{GetSpan, SPAN, Span};
use oxc_ti_compile::PassContext;
use oxc_traverse::{BoundIdentifier, Traverse, traverse_mut};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::{TraverseCtx, js_path::line_column};

const GLOBAL_NAME: &str = "DocInit";

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocInitOptions {
    #[serde(default)]
    base_id: u32,
}

#[derive(Serialize)]
pub struct Entry {
    id: u32,
    function: String,
    line: u32,
    column: u32,
}

pub struct DocInit {
    base_id: u32,
    /// The renamed callees' references, by which `after` finds the calls again.
    calls: FxHashSet<ReferenceId>,
    entries: Vec<Entry>,
}

/// `DocInit(...)` with no binding for `DocInit` in scope.
fn is_global_call(call: &CallExpression<'_>, scoping: &Scoping) -> bool {
    let Expression::Identifier(callee) = &call.callee else { return false };
    callee.name == GLOBAL_NAME && scoping.get_reference(callee.reference_id()).symbol_id().is_none()
}

struct FindGlobalCall<'s> {
    scoping: &'s Scoping,
    found: bool,
}

impl<'a> Visit<'a> for FindGlobalCall<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if is_global_call(call, self.scoping) {
            self.found = true;
        } else if !self.found {
            walk_call_expression(self, call);
        }
    }
}

impl DocInit {
    pub fn new(options: DocInitOptions) -> Self {
        Self { base_id: options.base_id, calls: FxHashSet::default(), entries: vec![] }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn has_calls(&self) -> bool {
        !self.calls.is_empty()
    }

    /// Inserts `import { docInit as _docInit } from 'tungsten'` when the file calls the global.
    pub fn insert_import<'a>(
        program: &mut Program<'a>,
        source_text: &str,
        ctx: &mut TraverseCtx<'a>,
    ) -> Option<BoundIdentifier<'a>> {
        // the substring gate keeps the pre-scan off the files that cannot need it
        if !source_text.contains(GLOBAL_NAME) {
            return None;
        }
        let mut finder = FindGlobalCall { scoping: ctx.scoping(), found: false };
        finder.visit_program(program);
        if !finder.found {
            return None;
        }
        let binding = ctx.generate_uid_in_root_scope("docInit", SymbolFlags::Import);
        let imported = ModuleExportName::new_identifier_name(SPAN, "docInit", ctx);
        let specifier = ImportDeclarationSpecifier::new_import_specifier(
            SPAN,
            imported,
            binding.create_binding_identifier(ctx),
            ImportOrExportKind::Value,
            ctx,
        );
        let import = Statement::new_import_declaration(
            SPAN,
            Some(ArenaVec::from_value_in(specifier, ctx)),
            StringLiteral::new(SPAN, "tungsten", None, ctx),
            None,
            None,
            ImportOrExportKind::Value,
            ctx,
        );
        program.body.insert(0, import);
        Some(binding)
    }

    pub fn claim_call<'a>(
        &mut self,
        call: &mut CallExpression<'a>,
        import: &BoundIdentifier<'a>,
        ctx: &mut TraverseCtx<'a>,
    ) {
        if !is_global_call(call, ctx.scoping()) {
            return;
        }
        let callee = import.create_spanned_read_expression(call.callee.span(), ctx);
        if let Expression::Identifier(identifier) = &callee {
            self.calls.insert(identifier.reference_id());
        }
        call.callee = callee;
    }

    #[expect(clippy::needless_pass_by_value, reason = "taken as `Passes::after` gives it")]
    pub fn after(&mut self, context: PassContext<'_, '_>, scoping: Scoping) -> Scoping {
        let mut after = After {
            doc_init: self,
            source_text: context.source_text,
            errors: context.errors,
            failed: false,
        };
        traverse_mut(&mut after, context.allocator, context.program, scoping, ())
    }

    /// Checks a claimed call, records its entry and gives it its id.
    fn rewrite<'a>(
        &mut self,
        call: &mut CallExpression<'a>,
        source_text: &str,
        ctx: &TraverseCtx<'a>,
    ) -> Result<(), OxcDiagnostic> {
        let function = match call.arguments.first() {
            Some(Argument::ArrowFunctionExpression(arrow)) => DocFunction {
                span: arrow.span,
                scope_id: arrow.scope_id(),
                is_async: arrow.r#async,
                is_generator: false,
                has_params: !arrow.params.is_empty(),
            },
            Some(Argument::FunctionExpression(func)) => DocFunction {
                span: func.span,
                scope_id: func.scope_id(),
                is_async: func.r#async,
                is_generator: func.generator,
                has_params: !func.params.is_empty(),
            },
            _ => {
                return Err(OxcDiagnostic::error(format!(
                    "{GLOBAL_NAME}() takes an inline function expression as its first argument - it is inlined into the document head at build, so it cannot be a reference"
                ))
                .with_label(call.span));
            }
        };
        if function.is_async || function.is_generator {
            return Err(OxcDiagnostic::error(format!(
                "a {GLOBAL_NAME}() function runs synchronously in the document head, so it cannot be async or a generator"
            ))
            .with_label(function.span));
        }
        if function.has_params {
            return Err(OxcDiagnostic::error(format!(
                "a {GLOBAL_NAME}() function takes no parameters - nothing calls it with any"
            ))
            .with_label(function.span));
        }
        if call.arguments.len() > 2 {
            return Err(OxcDiagnostic::error(format!(
                "{GLOBAL_NAME}() takes a function and an optional server value, nothing more"
            ))
            .with_label(call.span));
        }
        let expression = call.arguments[0].to_expression();
        let mut check = SelfContained {
            scoping: ctx.scoping(),
            function_scope: function.scope_id,
            plain_functions: 0,
            assignment_targets: FxHashSet::default(),
            error: None,
        };
        check.visit_expression(expression);
        if let Some(error) = check.error {
            return Err(error);
        }

        let mut codegen = Codegen::new();
        codegen.print_expression(expression);
        let (line, column) = line_column(source_text, function.span.start as usize);
        let id = self.base_id + u32::try_from(self.entries.len()).expect("fewer than 2^32 calls");
        self.entries.push(Entry {
            id,
            function: codegen.into_source_text(),
            line,
            column: column + 1,
        });
        call.arguments[0] =
            Argument::new_numeric_literal(SPAN, f64::from(id), None, NumberBase::Decimal, ctx);
        Ok(())
    }
}

struct DocFunction {
    span: Span,
    scope_id: ScopeId,
    is_async: bool,
    is_generator: bool,
    has_params: bool,
}

struct After<'p> {
    doc_init: &'p mut DocInit,
    source_text: &'p str,
    errors: &'p mut Vec<OxcDiagnostic>,
    /// Babel throws at the first bad call, so the rest go unchecked.
    failed: bool,
}

impl<'a> Traverse<'a, ()> for After<'_> {
    fn enter_call_expression(&mut self, call: &mut CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
        if self.failed {
            return;
        }
        let Expression::Identifier(callee) = &call.callee else { return };
        if !self.doc_init.calls.contains(&callee.reference_id()) {
            return;
        }
        if let Err(error) = self.doc_init.rewrite(call, self.source_text, ctx) {
            self.errors.push(error);
            self.failed = true;
        }
    }
}

/// The function can only run against the head's globals: anything it reaches from the module is gone.
struct SelfContained<'s> {
    scoping: &'s Scoping,
    function_scope: ScopeId,
    /// Non-arrow functions entered, counting the `DocInit` function itself: `this` in one is its own.
    plain_functions: u32,
    /// Identifiers an assignment writes to, which Babel's `ReferencedIdentifier` skips.
    assignment_targets: FxHashSet<ReferenceId>,
    error: Option<OxcDiagnostic>,
}

impl SelfContained<'_> {
    fn collect_targets(&mut self, target: &AssignmentTarget<'_>) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(identifier) => {
                self.assignment_targets.insert(identifier.reference_id());
            }
            AssignmentTarget::ArrayAssignmentTarget(array) => self.collect_array_targets(array),
            AssignmentTarget::ObjectAssignmentTarget(object) => self.collect_object_targets(object),
            _ => {}
        }
    }

    fn collect_array_targets(&mut self, array: &ArrayAssignmentTarget<'_>) {
        for element in array.elements.iter().flatten() {
            self.collect_maybe_default(element);
        }
        if let Some(rest) = &array.rest {
            self.collect_targets(&rest.target);
        }
    }

    fn collect_object_targets(&mut self, object: &ObjectAssignmentTarget<'_>) {
        for property in &object.properties {
            match property {
                AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(shorthand) => {
                    self.assignment_targets.insert(shorthand.binding.reference_id());
                }
                AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
                    self.collect_maybe_default(&property.binding);
                }
            }
        }
        if let Some(rest) = &object.rest {
            self.collect_targets(&rest.target);
        }
    }

    fn collect_maybe_default(&mut self, target: &AssignmentTargetMaybeDefault<'_>) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with_default) => {
                self.collect_targets(&with_default.binding);
            }
            _ => {
                if let Some(target) = target.as_assignment_target() {
                    self.collect_targets(target);
                }
            }
        }
    }

    fn is_inside_function(&self, scope_id: ScopeId) -> bool {
        self.scoping.scope_ancestors(scope_id).any(|ancestor| ancestor == self.function_scope)
    }
}

impl<'a> Visit<'a> for SelfContained<'_> {
    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        self.plain_functions += 1;
        walk_function(self, func, flags);
        self.plain_functions -= 1;
    }

    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        self.collect_targets(&expr.left);
        walk_assignment_expression(self, expr);
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if self.error.is_some() || self.assignment_targets.contains(&identifier.reference_id()) {
            return;
        }
        let Some(symbol_id) = self.scoping.get_reference(identifier.reference_id()).symbol_id()
        else {
            return; // a real global: document, localStorage, ...
        };
        if self.is_inside_function(self.scoping.symbol_scope_id(symbol_id)) {
            return;
        }
        self.error = Some(
            OxcDiagnostic::error(format!(
                "{GLOBAL_NAME}() function references '{}' from outside the function. It runs in the document head, where nothing from this module exists - only globals like document and localStorage are available",
                identifier.name
            ))
            .with_label(identifier.span),
        );
    }

    fn visit_this_expression(&mut self, this: &ThisExpression) {
        if self.error.is_none() && self.plain_functions == 0 {
            self.error = Some(
                OxcDiagnostic::error(format!(
                    "'this' in a {GLOBAL_NAME}() function refers to the enclosing scope, which does not exist in the document head"
                ))
                .with_label(this.span),
            );
        }
    }
}
