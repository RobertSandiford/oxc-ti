//! The AST half of `BabelTransformPackageImports`.
//!
//! The Babel visitor resolves each bare import and walks its package; that stays in JS, which reads
//! `bareImports`. Only a target without a JavaScript extension changes the AST, its source becoming
//! the target's web location, which JS passes back in `replacements`.

use oxc_ast::ast::{ImportDeclaration, Program, Statement};
use oxc_str::Str;
use rustc_hash::FxHashMap;
use serde::Deserialize;

use crate::{TraverseCtx, js_path::is_bare_import};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageImportsOptions {
    #[serde(default)]
    replacements: FxHashMap<String, String>,
}

pub struct PackageImports {
    replacements: FxHashMap<String, String>,
    /// The bare specifier each import had, by the start of its source string.
    seen: FxHashMap<u32, String>,
    /// Bare stylesheet imports `cssImports` replaced, by the start of their source string.
    converted: Vec<(u32, String)>,
    bare_imports: Vec<String>,
}

impl PackageImports {
    pub fn new(options: PackageImportsOptions) -> Self {
        Self {
            replacements: options.replacements,
            seen: FxHashMap::default(),
            converted: vec![],
            bare_imports: vec![],
        }
    }

    pub fn bare_imports(&self) -> &[String] {
        &self.bare_imports
    }

    pub fn visit_import<'a>(&mut self, decl: &mut ImportDeclaration<'a>, ctx: &TraverseCtx<'a>) {
        let Some(location) = decl.source.value.as_str() else { return };
        if !is_bare_import(location) {
            return;
        }
        // an inserted import has no position to find it by, but needs no replacement either
        if !decl.source.span.is_empty() {
            self.seen.insert(decl.source.span.start, location.to_string());
        }
        if let Some(replacement) = self.replacements.get(location) {
            decl.source.value = Str::from_str_in(replacement, ctx).into();
            decl.source.raw = None;
        }
    }

    pub fn import_converted(&mut self, source_start: u32) {
        if let Some(location) = self.seen.remove(&source_start) {
            self.converted.push((source_start, location));
        }
    }

    /// Lists the bare imports in the compiled program, as they were before any replacement.
    pub fn collect(&mut self, program: &Program<'_>) {
        let mut found = self.converted.clone();
        for stmt in &program.body {
            let Statement::ImportDeclaration(decl) = stmt else { continue };
            let start = decl.source.span.start;
            let location = if decl.source.span.is_empty() { None } else { self.seen.get(&start) };
            match (location, decl.source.value.as_str()) {
                (Some(location), _) => found.push((start, location.clone())),
                (None, Some(value)) if is_bare_import(value) => {
                    found.push((start, value.to_string()));
                }
                _ => {}
            }
        }
        found.sort_by_key(|(start, _)| *start);
        for (_, location) in found {
            if !self.bare_imports.contains(&location) {
                self.bare_imports.push(location);
            }
        }
    }
}
