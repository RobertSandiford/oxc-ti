//! `BabelTransformCssImports`: `import g from './g.scss'` becomes `const g = '/<dir>/g.css'`, the
//! stylesheet's path in the build relative to the code dir.

use std::path::Path;

use oxc_allocator::{ArenaBox, ArenaVec};
use oxc_ast::ast::{
    BindingPattern, Expression, ImportDeclaration, ImportDeclarationSpecifier, Statement,
    VariableDeclarationKind, VariableDeclarator,
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_semantic::{Reference, Scoping, SymbolFlags};
use oxc_span::SPAN;
use oxc_str::Str;
use serde::Deserialize;

use crate::{
    TraverseCtx,
    js_path::{dirname, is_bare_import, slash, slice_utf16},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CssImportsOptions {
    abs_code_dir: String,
    abs_source_dir: String,
}

pub struct CssImports {
    options: CssImportsOptions,
}

impl CssImports {
    pub fn new(options: CssImportsOptions) -> Self {
        Self { options }
    }

    /// Replaces a stylesheet import, returning the start of its source string when it did.
    pub fn transform<'a>(
        &self,
        stmt: &mut Statement<'a>,
        file: &str,
        typescript: bool,
        errors: &mut Vec<OxcDiagnostic>,
        ctx: &mut TraverseCtx<'a>,
    ) -> Option<u32> {
        let Statement::ImportDeclaration(decl) = stmt else { return None };
        let import_target = decl.source.value.as_str()?;
        if ![".css", ".scss", ".sass"].iter().any(|extension| import_target.ends_with(extension)) {
            return None;
        }
        // Babel's typescript preset removes these at `Program` enter, before any plugin visits them
        if typescript && typescript_removes(decl, ctx.scoping()) {
            return None;
        }

        let location = self.code_dir_location(import_target, file);
        if !is_bare_import(import_target) {
            let css_path = format!("{}{location}", self.options.abs_source_dir);
            if !Path::new(&css_path).exists() {
                errors.push(
                    OxcDiagnostic::error(format!("Could not find stylesheet '{import_target}'"))
                        .with_label(decl.source.span),
                );
                return None;
            }
        }

        let local = match decl.specifiers.as_ref().map(|specifiers| specifiers.as_slice()) {
            None | Some([]) => {
                errors.push(OxcDiagnostic::warn(format!(
                    "Tungsten BabelTransformCssImports Error: CSS file '{location}' is not being imported to a variable.\nCSS files should be imported like `import globalStyles from './style/global.css'`\nThen used in a <Stylesheet> component like `<Stylesheet>{{globalStyles}}</Stylesheet>`"
                )).with_label(decl.span));
                return None;
            }
            Some([specifier]) => specifier_local(specifier).clone(),
            Some(_) => {
                errors.push(OxcDiagnostic::warn(format!(
                    "Tungsten BabelTransformCssImports Error: The import of CSS file '{location}' has too any specifiers.\nOnly 1 specifier should be used, e.g. `import globalStyles from './style/global.css'`"
                )).with_label(decl.span));
                return None;
            }
        };

        let stem = location.rfind('.').map_or(location.as_str(), |dot| &location[..dot]);
        let value = Str::from_strs_array_in([stem, ".css"], ctx);
        *ctx.scoping_mut().symbol_flags_mut(local.symbol_id()) =
            SymbolFlags::BlockScopedVariable | SymbolFlags::ConstVariable;
        let source_start = decl.source.span.start;
        let declarator = VariableDeclarator::new(
            SPAN,
            BindingPattern::BindingIdentifier(ArenaBox::new_in(local, ctx)),
            None,
            Some(Expression::new_string_literal(SPAN, value, None, ctx)),
            false,
            ctx,
        );
        *stmt = Statement::new_variable_declaration(
            decl.span,
            VariableDeclarationKind::Const,
            ArenaVec::from_value_in(declarator, ctx),
            false,
            ctx,
        );
        Some(source_start)
    }

    /// Where a relative import's stylesheet sits under the code dir, as `/dir/file.ext`.
    fn code_dir_location(&self, import_target: &str, file: &str) -> String {
        let relative = || {
            let file_rel_dir =
                slash(dirname(slice_utf16(file, self.options.abs_code_dir.encode_utf16().count())));
            if file_rel_dir == "/" { String::new() } else { file_rel_dir }
        };
        if import_target.starts_with("./") {
            format!("{}{}", relative(), &import_target[1..])
        } else if import_target.starts_with("../") {
            format!("{}/{import_target}", relative())
        } else {
            import_target.to_string()
        }
    }
}

fn specifier_local<'s, 'a>(
    specifier: &'s ImportDeclarationSpecifier<'a>,
) -> &'s oxc_ast::ast::BindingIdentifier<'a> {
    match specifier {
        ImportDeclarationSpecifier::ImportSpecifier(it) => &it.local,
        ImportDeclarationSpecifier::ImportDefaultSpecifier(it) => &it.local,
        ImportDeclarationSpecifier::ImportNamespaceSpecifier(it) => &it.local,
    }
}

/// Whether the TypeScript transform will remove the whole import: it is `import type`, or each
/// specifier is a type or has no value reference.
fn typescript_removes(decl: &ImportDeclaration<'_>, scoping: &Scoping) -> bool {
    if decl.import_kind.is_type() {
        return true;
    }
    let Some(specifiers) = &decl.specifiers else { return false };
    !specifiers.is_empty()
        && specifiers.iter().all(|specifier| {
            if let ImportDeclarationSpecifier::ImportSpecifier(it) = specifier
                && it.import_kind.is_type()
            {
                return true;
            }
            let symbol_id = specifier_local(specifier).symbol_id();
            (scoping.symbol_flags(symbol_id) - SymbolFlags::Import).is_value()
                || scoping.get_resolved_references(symbol_id).all(Reference::is_type)
        })
}
