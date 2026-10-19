//! The AST half of `BabelTransformComponentStyles`.
//!
//! Compiling a template's styles (Sass, postcss name scoping) needs JS, so a file takes two compiles:
//! the first lists the templates, JS compiles them, and the second substitutes the results. Babel
//! replaces a template with
//! `{ "moduleName": ..., "tags": {...}, "classes": {...}, "keyframes": {...} }` and, when it is inside
//! a class, appends `static $stylesheet = 'i/<module>.css'` to the nearest enclosing class body.

use indexmap::IndexMap;
use oxc_allocator::ArenaVec;
use oxc_ast::ast::{
    ClassBody, ClassElement, Expression, ObjectPropertyKind, PropertyDefinitionType, PropertyKey,
    PropertyKind, TaggedTemplateExpression,
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::SPAN;
use oxc_str::Str;
use serde::{Deserialize, Serialize};

use crate::TraverseCtx;

const TAGS: [&str; 4] = ["Css", "Sass", "Scss", "ReactZoneScss"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentStylesOptions {
    results: Option<Vec<StyleResult>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyleResult {
    module_name: String,
    tags: IndexMap<String, String>,
    classes: IndexMap<String, String>,
    keyframes: IndexMap<String, String>,
    stylesheet: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedStyle {
    index: u32,
    tag: &'static str,
    raw: String,
    in_class: bool,
}

pub struct ComponentStyles {
    results: Option<Vec<StyleResult>>,
    extracted: Vec<ExtractedStyle>,
    /// Templates being listed that enclose the current node: Babel never reaches a template inside one.
    listing_depth: u32,
    /// A list of `$stylesheet` values to append per class body being traversed.
    class_bodies: Vec<Vec<String>>,
}

/// The tag of a style template, if `expr` is one.
fn style_tag(expr: &Expression<'_>) -> Option<&'static str> {
    let Expression::TaggedTemplateExpression(tagged) = expr else { return None };
    let Expression::Identifier(tag) = &tagged.tag else { return None };
    TAGS.into_iter().find(|name| tag.name == *name)
}

impl ComponentStyles {
    pub fn new(options: ComponentStylesOptions) -> Self {
        Self { results: options.results, extracted: vec![], listing_depth: 0, class_bodies: vec![] }
    }

    pub fn extracted(&self) -> &[ExtractedStyle] {
        &self.extracted
    }

    pub fn enter_expression<'a>(&mut self, expr: &mut Expression<'a>, ctx: &TraverseCtx<'a>) {
        if self.listing_depth > 0 {
            return;
        }
        let Some(tag) = style_tag(expr) else { return };
        let Expression::TaggedTemplateExpression(tagged) = expr else { return };
        let index = self.extracted.len();
        self.extracted.push(ExtractedStyle {
            index: u32::try_from(index + 1).expect("fewer than 2^32 templates"),
            tag,
            raw: first_quasi(tagged).to_string(),
            in_class: !self.class_bodies.is_empty(),
        });
        let Some(result) = self.results.as_ref().and_then(|results| results.get(index)) else {
            self.listing_depth += 1;
            return;
        };
        if let Some(stylesheets) = self.class_bodies.last_mut() {
            stylesheets.push(result.stylesheet.clone());
        }
        *expr = style_object(result, ctx);
    }

    pub fn exit_expression(&mut self, expr: &Expression<'_>) {
        if self.listing_depth > 0 && style_tag(expr).is_some() {
            self.listing_depth -= 1;
        }
    }

    pub fn enter_class_body(&mut self) {
        self.class_bodies.push(vec![]);
    }

    pub fn exit_class_body<'a>(&mut self, body: &mut ClassBody<'a>, ctx: &TraverseCtx<'a>) {
        let stylesheets = self.class_bodies.pop().unwrap_or_default();
        for stylesheet in stylesheets {
            let key = PropertyKey::new_static_identifier(SPAN, "$stylesheet", ctx);
            let value =
                Expression::new_string_literal(SPAN, Str::from_str_in(&stylesheet, ctx), None, ctx);
            body.body.push(ClassElement::new_property_definition(
                SPAN,
                PropertyDefinitionType::PropertyDefinition,
                ArenaVec::new_in(ctx),
                key,
                None,
                Some(value),
                false,
                true,
                false,
                false,
                false,
                false,
                false,
                None,
                ctx,
            ));
        }
    }

    pub fn check_results(&self, errors: &mut Vec<OxcDiagnostic>) {
        let Some(results) = &self.results else { return };
        if results.len() != self.extracted.len() {
            errors.push(OxcDiagnostic::error(format!(
                "componentStyles: the file has {} style templates but {} results were given",
                self.extracted.len(),
                results.len()
            )));
        }
    }
}

/// Babel reads only `quasis[0].value.raw`, so `${...}` and what follows are dropped.
fn first_quasi<'a>(tagged: &TaggedTemplateExpression<'a>) -> &'a str {
    tagged.quasi.quasis.first().map_or("", |quasi| quasi.value.raw.as_str())
}

fn style_object<'a>(result: &StyleResult, ctx: &TraverseCtx<'a>) -> Expression<'a> {
    let string =
        |text: &str| Expression::new_string_literal(SPAN, Str::from_str_in(text, ctx), None, ctx);
    let property = |key: &str, value: Expression<'a>| {
        let key = PropertyKey::new_string_literal(SPAN, Str::from_str_in(key, ctx), None, ctx);
        ObjectPropertyKind::new_object_property(
            SPAN,
            PropertyKind::Init,
            key,
            value,
            false,
            false,
            false,
            ctx,
        )
    };
    let names = |names: &IndexMap<String, String>| {
        let properties = ArenaVec::from_iter_in(
            names.iter().map(|(key, value)| property(key, string(value))),
            ctx,
        );
        Expression::new_object_expression(SPAN, properties, ctx)
    };
    let properties = ArenaVec::from_array_in(
        [
            property("moduleName", string(&result.module_name)),
            property("tags", names(&result.tags)),
            property("classes", names(&result.classes)),
            property("keyframes", names(&result.keyframes)),
        ],
        ctx,
    );
    Expression::new_object_expression(SPAN, properties, ctx)
}
