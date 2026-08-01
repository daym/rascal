use std::{env, fs, path::PathBuf, process::ExitCode};

use rascal::pascal_parser;

fn run() -> Result<bool, String> {
    let mut arguments = env::args().skip(1);
    let path = PathBuf::from(
        arguments
            .next()
            .ok_or_else(|| "usage: rascal <source.pp>".to_owned())?,
    );
    if let Some(extra) = arguments.next() {
        return Err(format!("unexpected argument `{extra}`"));
    }
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let output = pascal_parser::parse(&source);

    for diagnostic in &output.diagnostics {
        eprintln!(
            "{}:{}..{}: {}",
            path.display(),
            diagnostic.span.start,
            diagnostic.span.end,
            diagnostic.message
        );
    }
    let Some(file) = output.file else {
        return Ok(false);
    };
    println!(
        "{:?} {}: {} CST nodes, {} sections",
        file.kind,
        file.name.as_deref().unwrap_or("<anonymous>"),
        file.nodes.len(),
        file.sections.len()
    );
    for section in &file.sections {
        println!(
            "  {:?}: nodes {}..{}, bytes {}..{}",
            section.kind,
            section.nodes.start,
            section.nodes.end,
            section.span.start,
            section.span.end
        );
    }
    Ok(output.diagnostics.is_empty())
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("rascal: {error}");
            ExitCode::from(2)
        }
    }
}
