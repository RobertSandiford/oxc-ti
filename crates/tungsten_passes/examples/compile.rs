#![expect(clippy::print_stdout)]
//! Compiles files with [`TungstenPasses`], for scripts to call before a Node binding exists.
//!
//! Reads a JSON array of jobs from stdin, `[{ "path": "...", "source"?: "...", "options": "{...}" }]`,
//! where `options` is the passes' options JSON, as a string so its key order survives, and `source`
//! defaults to the file's content. Prints a JSON
//! array of `{ "code", "output", "errors": [{ "severity", "message" }] }`, one per job.
//!
//! Usage: `cargo run -p tungsten_passes --example compile < jobs.json`

use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use oxc_diagnostics::Severity;
use oxc_ti_compile::{CompileOptions, compile};
use serde::Deserialize;
use serde_json::{Value, json};
use tungsten_passes::TungstenPasses;

#[derive(Deserialize)]
struct Job {
    path: String,
    source: Option<String>,
    options: String,
}

fn run(job: &Job) -> Value {
    let source = match &job.source {
        Some(source) => source.clone(),
        None => match fs::read_to_string(&job.path) {
            Ok(source) => source,
            Err(error) => {
                return json!({ "errors": [{ "severity": "error", "message": error.to_string() }] });
            }
        },
    };
    let mut passes = match TungstenPasses::from_options_json(&job.options) {
        Ok(passes) => passes,
        Err(error) => return json!({ "errors": [{ "severity": "error", "message": error }] }),
    };
    let options =
        CompileOptions { jsx_import_source: Some("tungsten".to_string()), source_map: false };
    let ret = compile(Path::new(&job.path), &source, &options, &mut passes);
    let errors: Vec<_> = ret
        .errors
        .iter()
        .map(|error| {
            let severity = if error.severity == Severity::Warning { "warning" } else { "error" };
            json!({ "severity": severity, "message": error.message })
        })
        .collect();
    let output: Value = serde_json::from_str(&passes.output_json()).expect("output is JSON");
    json!({ "code": ret.code, "output": output, "errors": errors })
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("stdin is readable");
    let jobs: Vec<Job> = serde_json::from_str(&input).expect("stdin is a JSON array of jobs");
    let results: Vec<_> = jobs.iter().map(run).collect();
    println!("{}", Value::Array(results));
}
