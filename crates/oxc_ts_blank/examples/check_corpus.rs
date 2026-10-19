//! Blanks every TypeScript file under the given roots and checks the output against oxc's transformer.
//!
//! For each file it checks that the blanked output parses as JavaScript, keeps every line's length,
//! and, for non-JSX files, prints the same minified code as the transformer set to remove only
//! explicit type imports.
//!
//! Usage: cargo run -p oxc_ts_blank --example check_corpus -- <out dir> <root>...

#![expect(clippy::print_stdout)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use oxc_allocator::{Allocator, Vec as ArenaVec};
use oxc_ast::ast::{Program, Statement};
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use oxc_ts_blank::blank;

fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = PathBuf::from(args.next().expect("usage: check_corpus <out dir> <root>..."));
    fs::create_dir_all(&out_dir).unwrap();
    let mut files = vec![];
    for root in args {
        collect(Path::new(&root), &mut files);
    }

    let mut checked = 0;
    let mut same = 0;
    let mut not_compared = 0;
    let mut unparseable = 0;
    let mut unsupported: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    let mut problems = vec![];
    for (n, path) in files.iter().enumerate() {
        let source = fs::read_to_string(path).unwrap();
        let source_type = SourceType::from_path(path).unwrap();
        let allocator = Allocator::default();
        if !Parser::new(&allocator, &source, source_type).parse().diagnostics.is_empty() {
            unparseable += 1;
            continue;
        }
        checked += 1;
        let ret = blank(&source, source_type);
        if !ret.errors.is_empty() {
            for error in ret.errors {
                let paths = unsupported.entry(error.to_string()).or_default();
                if paths.last() != Some(path) {
                    paths.push(path.clone());
                }
            }
            continue;
        }
        let save = |suffix: &str, text: &str| {
            let name = format!("{n}-{}{suffix}", path.file_name().unwrap().to_string_lossy());
            let saved = out_dir.join(name);
            fs::write(&saved, text).unwrap();
            saved
        };
        if let Some(line) = changed_line(&source, &ret.code) {
            problems.push(format!(
                "{}: line {line} changed length ({})",
                path.display(),
                save(".js", &ret.code).display()
            ));
            continue;
        }
        let js_type = source_type.with_typescript(false);
        let Some(blanked) = minified_js(&ret.code, js_type) else {
            problems.push(format!(
                "{}: output is not valid JS ({})",
                path.display(),
                save(".js", &ret.code).display()
            ));
            continue;
        };
        if source_type.is_jsx() {
            not_compared += 1;
            continue;
        }
        let transformed = transformed(&source, path, source_type);
        if blanked == transformed {
            same += 1;
        } else {
            save(".blanked.min.js", &blanked);
            let saved = save(".transformed.min.js", &transformed);
            problems.push(format!(
                "{}: differs from the transformer ({})",
                path.display(),
                saved.display()
            ));
        }
    }

    println!(
        "files: {}  unparseable: {unparseable}  checked: {checked}  same as transformer: {same}  JSX not compared: {not_compared}  problems: {}",
        files.len(),
        problems.len()
    );
    for (message, paths) in &unsupported {
        println!("unsupported in {} files: {message}", paths.len());
        for path in paths.iter().take(3) {
            println!("    {}", path.display());
        }
    }
    for problem in &problems {
        println!("{problem}");
    }
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if path.is_dir() {
            if name != "node_modules" && name != "dist" {
                collect(&path, files);
            }
        } else if [".ts", ".tsx", ".mts", ".cts"].iter().any(|ext| name.ends_with(ext))
            && ![".d.ts", ".d.mts", ".d.cts"].iter().any(|ext| name.ends_with(ext))
        {
            files.push(path);
        }
    }
}

fn changed_line(source: &str, code: &str) -> Option<usize> {
    let source_lines: Vec<_> = source.split('\n').collect();
    let code_lines: Vec<_> = code.split('\n').collect();
    if source_lines.len() != code_lines.len() {
        return Some(source_lines.len().min(code_lines.len()));
    }
    source_lines
        .iter()
        .zip(&code_lines)
        .position(|(a, b)| a.encode_utf16().count() != b.encode_utf16().count())
        .map(|i| i + 1)
}

struct RemoveEmptyStatements;

impl<'a> VisitMut<'a> for RemoveEmptyStatements {
    fn visit_statements(&mut self, statements: &mut ArenaVec<'a, Statement<'a>>) {
        statements.retain(|statement| !matches!(statement, Statement::EmptyStatement(_)));
        walk_mut::walk_statements(self, statements);
    }
}

/// Minified code without the `;` the blanker leaves for erased statements, or the `export {}`
/// that marks an emptied module, which only the transformer adds.
#[expect(
    clippy::disallowed_methods,
    reason = "a test tool, where an extra allocation doesn't matter"
)]
fn minify(program: &mut Program<'_>) -> String {
    RemoveEmptyStatements.visit_program(program);
    let code = Codegen::new().with_options(CodegenOptions::minify()).build(program).code;
    code.replace("export{};", "")
}

fn minified_js(code: &str, source_type: SourceType) -> Option<String> {
    let allocator = Allocator::default();
    let mut ret = Parser::new(&allocator, code, source_type).parse();
    ret.diagnostics.is_empty().then(|| minify(&mut ret.program))
}

fn transformed(source: &str, path: &Path, source_type: SourceType) -> String {
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, source, source_type).parse().program;
    let scoping = SemanticBuilder::new().build(&program).semantic.into_scoping();
    let mut options = TransformOptions::from_target("esnext").unwrap();
    options.typescript.only_remove_type_imports = true;
    Transformer::new(&allocator, path, &options).build_with_scoping(scoping, &mut program);
    minify(&mut program)
}
