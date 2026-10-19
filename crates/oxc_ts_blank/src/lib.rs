//! Erases TypeScript and TightScript type syntax by overwriting it with whitespace.
//!
//! Every line and column stays where it was, so the output runs without a source map.
//! Like Node's strip-only mode, syntax that needs a real transform (enums, namespaces with
//! values, parameter properties, `import =` and `export =`) is reported as an error.

#![expect(
    clippy::cast_possible_truncation,
    reason = "oxc's parser rejects sources over u32::MAX bytes"
)]

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::FxHashMap;

pub struct BlankResult {
    /// The JavaScript, the same length as the source in UTF-16 code units, line for line.
    pub code: String,
    pub errors: Vec<OxcDiagnostic>,
}

pub fn blank(source_text: &str, source_type: SourceType) -> BlankResult {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source_text, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return BlankResult { code: String::new(), errors: parsed.diagnostics.to_vec() };
    }
    let mut blanker = Blanker::new(source_text);
    blanker.visit_program(&parsed.program);
    let errors = std::mem::take(&mut blanker.errors);
    BlankResult { code: blanker.render(), errors }
}

struct Blanker<'s> {
    source: &'s str,
    /// Byte ranges to overwrite with spaces.
    ranges: Vec<(u32, u32)>,
    /// The start of the widest range ending at each offset.
    range_starts_by_end: FxHashMap<u32, u32>,
    /// Characters to write in place of a blanked character.
    replacements: FxHashMap<u32, char>,
    errors: Vec<OxcDiagnostic>,
}

impl<'s> Blanker<'s> {
    fn new(source: &'s str) -> Self {
        Self {
            source,
            ranges: vec![],
            range_starts_by_end: FxHashMap::default(),
            replacements: FxHashMap::default(),
            errors: vec![],
        }
    }

    fn blank(&mut self, start: u32, end: u32) {
        if start >= end {
            return;
        }
        self.ranges.push((start, end));
        let widest = self.range_starts_by_end.entry(end).or_insert(start);
        *widest = (*widest).min(start);
    }

    fn blank_span(&mut self, span: Span) {
        self.blank(span.start, span.end);
    }

    /// Blanks a whole statement or class member, leaving a `;` so the code either side stays apart.
    fn blank_statement(&mut self, span: Span) {
        self.blank_span(span);
        self.replacements.insert(span.start, ';');
    }

    /// Blanks a list item and the comma after it.
    fn blank_with_comma(&mut self, span: Span) {
        let after = self.skip_trivia(span.end);
        let end = if self.byte(after) == Some(b',') { after + 1 } else { span.end };
        self.blank(span.start, end);
    }

    fn blank_type_specifiers(&mut self, specifiers: &[ExportSpecifier<'_>]) {
        for specifier in specifiers {
            if specifier.export_kind.is_type() {
                self.blank_with_comma(specifier.span);
            }
        }
    }

    fn error(&mut self, message: &'static str, span: Span) {
        self.errors.push(OxcDiagnostic::error(message).with_label(span));
    }

    /// Ends the statement with a `;` when its last tokens were blanked and nothing else ends it.
    fn keep_statement_end(&mut self, span: Span) {
        if span.end == 0 || self.byte(span.end - 1) == Some(b';') {
            return;
        }
        if let Some(&start) = self.range_starts_by_end.get(&span.end) {
            self.replacements.entry(start).or_insert(';');
        }
    }

    fn byte(&self, pos: u32) -> Option<u8> {
        self.source.as_bytes().get(pos as usize).copied()
    }

    /// The offset of the next token at or after `pos`, skipping whitespace and comments.
    fn skip_trivia(&self, mut pos: u32) -> u32 {
        loop {
            let rest = &self.source[pos as usize..];
            let Some(c) = rest.chars().next() else { return pos };
            let skipped = if c.is_whitespace() {
                c.len_utf8()
            } else if rest.starts_with("//") {
                rest.find(['\n', '\r', '\u{2028}', '\u{2029}']).unwrap_or(rest.len())
            } else if let Some(comment) = rest.strip_prefix("/*") {
                comment.find("*/").map_or(rest.len(), |i| i + 4)
            } else {
                return pos;
            };
            pos += skipped as u32;
        }
    }

    /// The identifier-like word starting at `pos`, if any.
    fn word_at(&self, pos: u32) -> Option<&'s str> {
        let rest = &self.source[pos as usize..];
        let len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
            .unwrap_or(rest.len());
        (len > 0).then(|| &rest[..len])
    }

    /// Blanks the modifier words between `start` and a class member's key, such as `private`,
    /// `readonly` and TightScript's permission words, keeping JavaScript's own (`static`, `get`).
    fn blank_member_modifiers(&mut self, start: u32, key_start: u32, has_permission: bool) {
        let first = self.skip_trivia(start);
        let mut pos = first;
        while pos < key_start {
            let Some(word) = self.word_at(pos) else { break };
            let end = pos + word.len() as u32;
            let is_ts_modifier = matches!(
                word,
                "public"
                    | "private"
                    | "protected"
                    | "readonly"
                    | "override"
                    | "declare"
                    | "abstract"
            );
            let is_permission =
                has_permission && TSTypeOperatorOperator::permission_from_str(word).is_some();
            if is_ts_modifier || is_permission {
                self.blank(pos, end);
                // The member would otherwise start with its key, which may be `[` or `*`.
                if pos == first {
                    self.replacements.insert(pos, ';');
                }
            }
            pos = self.skip_trivia(end);
        }
    }

    /// Blanks the `?` or `!` after a name, as in `x?: T` or `x!: T`.
    fn blank_marker_after(&mut self, name_end: u32, computed: bool, marker: u8) {
        let mut pos = self.skip_trivia(name_end);
        if computed && self.byte(pos) == Some(b']') {
            pos = self.skip_trivia(pos + 1);
        }
        if self.byte(pos) == Some(marker) {
            self.blank(pos, pos + 1);
        }
    }

    fn members_start(span: Span, decorators: &[Decorator<'_>]) -> u32 {
        decorators.last().map_or(span.start, |d| d.span.end)
    }

    /// Whether the statement exists only at the type level, reporting the kinds Node's strip-only
    /// mode rejects. Those are blanked too, so one error doesn't hide the rest.
    fn is_erasable_statement(&mut self, stmt: &Statement<'_>) -> bool {
        match stmt {
            Statement::ImportDeclaration(d) => d.import_kind.is_type(),
            Statement::ExportAllDeclaration(d) => d.export_kind.is_type(),
            Statement::ExportNamedDeclaration(d) => d.export_kind.is_type(),
            Statement::ExportFromDeclaration(d) => d.export_kind.is_type(),
            Statement::ExportDeclaration(d) => self.is_erasable_declaration(&d.declaration),
            Statement::ExportDefaultDeclaration(d) => match &d.declaration {
                ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => true,
                ExportDefaultDeclarationKind::FunctionDeclaration(f) => f.body.is_none(),
                _ => false,
            },
            Statement::TSExportAssignment(d) => {
                self.error("`export =` is not supported in strip-only mode", d.span);
                true
            }
            Statement::TSNamespaceExportDeclaration(_) => true,
            _ => stmt.as_declaration().is_some_and(|decl| self.is_erasable_declaration(decl)),
        }
    }

    fn is_erasable_declaration(&mut self, decl: &Declaration<'_>) -> bool {
        match decl {
            Declaration::VariableDeclaration(d) => d.declare,
            Declaration::ClassDeclaration(c) => c.declare,
            Declaration::FunctionDeclaration(f) => f.declare || f.body.is_none(),
            Declaration::TSTypeAliasDeclaration(_)
            | Declaration::TSInterfaceDeclaration(_)
            | Declaration::TSExternalModuleDeclaration(_)
            | Declaration::TSGlobalDeclaration(_)
            | Declaration::TSProvideDeclaration(_)
            | Declaration::TSTypeFromDeclaration(_) => true,
            Declaration::TSEnumDeclaration(e) => {
                if !e.declare {
                    self.error("TypeScript enum is not supported in strip-only mode", e.span);
                }
                true
            }
            Declaration::TSNamespaceDeclaration(n) => {
                if !n.declare && namespace_has_values(n) {
                    self.error(
                        "TypeScript namespace with values is not supported in strip-only mode",
                        n.span,
                    );
                }
                true
            }
            Declaration::TSImportEqualsDeclaration(d) => {
                if !d.import_kind.is_type() {
                    self.error("`import =` is not supported in strip-only mode", d.span);
                }
                true
            }
        }
    }

    fn render(mut self) -> String {
        self.ranges.sort_unstable();
        let mut out = String::with_capacity(self.source.len());
        let mut copied = 0usize;
        for &(start, end) in &self.ranges {
            let (start, end) = (start as usize, end as usize);
            if end <= copied {
                continue;
            }
            let start = start.max(copied);
            out.push_str(&self.source[copied..start]);
            for (i, c) in self.source[start..end].char_indices() {
                if matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                    out.push(c);
                    continue;
                }
                let mut width = c.len_utf16();
                if let Some(&replacement) = self.replacements.get(&((start + i) as u32)) {
                    out.push(replacement);
                    width -= 1;
                }
                out.extend(std::iter::repeat_n(' ', width));
            }
            copied = end;
        }
        out.push_str(&self.source[copied..]);
        out
    }
}

/// TypeScript's rule for an instantiated namespace: anything other than types inside it.
fn namespace_has_values(namespace: &TSNamespaceDeclaration<'_>) -> bool {
    match &namespace.body {
        TSNamespaceDeclarationBody::TSNamespaceDeclaration(inner) => namespace_has_values(inner),
        TSNamespaceDeclarationBody::TSModuleBlock(block) => block.body.iter().any(|stmt| {
            let decl = match stmt {
                Statement::EmptyStatement(_) => return false,
                Statement::ExportDeclaration(export) => &export.declaration,
                _ => match stmt.as_declaration() {
                    Some(decl) => decl,
                    None => return true,
                },
            };
            match decl {
                Declaration::TSNamespaceDeclaration(inner) => {
                    !inner.declare && namespace_has_values(inner)
                }
                Declaration::TSImportEqualsDeclaration(d) => !d.import_kind.is_type(),
                _ => !(decl.is_type() || decl.declare()),
            }
        }),
    }
}

impl<'a> Visit<'a> for Blanker<'_> {
    fn visit_statement(&mut self, stmt: &Statement<'a>) {
        if self.is_erasable_statement(stmt) {
            self.blank_statement(stmt.span());
            return;
        }
        walk::walk_statement(self, stmt);
        self.keep_statement_end(stmt.span());
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        for specifier in it.specifiers.iter().flatten() {
            if let ImportDeclarationSpecifier::ImportSpecifier(s) = specifier
                && s.import_kind.is_type()
            {
                self.blank_with_comma(s.span);
            }
        }
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        self.blank_type_specifiers(&it.specifiers);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.blank_type_specifiers(&it.specifiers);
    }

    fn visit_ts_type_annotation(&mut self, it: &TSTypeAnnotation<'a>) {
        self.blank_span(it.span);
    }

    fn visit_ts_type_parameter_declaration(&mut self, it: &TSTypeParameterDeclaration<'a>) {
        self.blank_span(it.span);
    }

    fn visit_ts_type_parameter_instantiation(&mut self, it: &TSTypeParameterInstantiation<'a>) {
        self.blank_span(it.span);
    }

    fn visit_ts_this_parameter(&mut self, it: &TSThisParameter<'a>) {
        self.blank_with_comma(it.span);
    }

    fn visit_ts_as_expression(&mut self, it: &TSAsExpression<'a>) {
        self.blank(it.expression.span().end, it.span.end);
        self.visit_expression(&it.expression);
    }

    fn visit_ts_satisfies_expression(&mut self, it: &TSSatisfiesExpression<'a>) {
        self.blank(it.expression.span().end, it.span.end);
        self.visit_expression(&it.expression);
    }

    fn visit_ts_non_null_expression(&mut self, it: &TSNonNullExpression<'a>) {
        self.blank(it.expression.span().end, it.span.end);
        self.visit_expression(&it.expression);
    }

    fn visit_ts_type_assertion(&mut self, it: &TSTypeAssertion<'a>) {
        self.blank(it.span.start, it.expression.span().start);
        self.visit_expression(&it.expression);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if it.definite {
            self.blank_marker_after(it.id.span().end, false, b'!');
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        if it.accessibility.is_some() || it.readonly || it.r#override {
            self.error(
                "TypeScript parameter property is not supported in strip-only mode",
                it.span,
            );
        }
        if it.optional {
            self.blank_marker_after(it.pattern.span().end, false, b'?');
        }
        walk::walk_formal_parameter(self, it);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        walk::walk_arrow_function_expression(self, it);
        // JavaScript allows no line break between `)` and `=>`, so the `)` moves to the end of a
        // return type that spans lines.
        if let Some(return_type) = &it.return_type {
            let gap = &self.source[it.params.span.end as usize..return_type.span.end as usize];
            let close_paren = it.params.span.end - 1;
            if gap.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
                && self.byte(close_paren) == Some(b')')
            {
                self.blank(close_paren, close_paren + 1);
                let type_text = &self.source[..return_type.span.end as usize];
                let last_char = type_text.chars().next_back().map_or(1, char::len_utf8) as u32;
                self.replacements.insert(return_type.span.end - last_char, ')');
            }
        }
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let mut pos = self.skip_trivia(Self::members_start(it.span, &it.decorators));
        let mut class_keyword_end = pos;
        while let Some(word) = self.word_at(pos) {
            let end = pos + word.len() as u32;
            if word == "class" {
                class_keyword_end = end;
                break;
            }
            if word == "abstract" {
                self.blank(pos, end);
            }
            pos = self.skip_trivia(end);
        }
        if let Some(last) = it.implements.last() {
            let after_heritage = [
                Some(class_keyword_end),
                it.id.as_ref().map(|id| id.span.end),
                it.type_parameters.as_ref().map(|t| t.span.end),
                it.heritage.as_ref().map(|h| h.expression.span().end),
                it.heritage.as_ref().and_then(|h| h.type_arguments.as_ref()).map(|t| t.span.end),
            ]
            .into_iter()
            .flatten()
            .max()
            .unwrap_or(class_keyword_end);
            let keyword = self.skip_trivia(after_heritage);
            if self.word_at(keyword) == Some("implements") {
                self.blank(keyword, last.span.end);
            }
        }
        walk::walk_class(self, it);
    }

    fn visit_class_element(&mut self, it: &ClassElement<'a>) {
        match it {
            ClassElement::TSIndexSignature(s) => self.blank_statement(s.span),
            ClassElement::PropertyDefinition(p) => {
                if p.declare || p.r#type == PropertyDefinitionType::TSAbstractPropertyDefinition {
                    self.blank_statement(p.span);
                    return;
                }
                let start = Self::members_start(p.span, &p.decorators);
                self.blank_member_modifiers(start, p.key.span().start, p.permission.is_some());
                if p.optional {
                    self.blank_marker_after(p.key.span().end, p.computed, b'?');
                }
                if p.definite {
                    self.blank_marker_after(p.key.span().end, p.computed, b'!');
                }
                walk::walk_property_definition(self, p);
                self.keep_statement_end(p.span);
            }
            ClassElement::AccessorProperty(p) => {
                if p.r#type == AccessorPropertyType::TSAbstractAccessorProperty {
                    self.blank_statement(p.span);
                    return;
                }
                let start = Self::members_start(p.span, &p.decorators);
                self.blank_member_modifiers(start, p.key.span().start, p.permission.is_some());
                if p.definite {
                    self.blank_marker_after(p.key.span().end, p.computed, b'!');
                }
                walk::walk_accessor_property(self, p);
                self.keep_statement_end(p.span);
            }
            ClassElement::MethodDefinition(m) => {
                if m.r#type == MethodDefinitionType::TSAbstractMethodDefinition
                    || m.value.body.is_none()
                {
                    self.blank_statement(m.span);
                    return;
                }
                let start = Self::members_start(m.span, &m.decorators);
                self.blank_member_modifiers(start, m.key.span().start, false);
                if m.optional {
                    self.blank_marker_after(m.key.span().end, m.computed, b'?');
                }
                walk::walk_method_definition(self, m);
            }
            ClassElement::StaticBlock(_) => walk::walk_class_element(self, it),
        }
    }
}
