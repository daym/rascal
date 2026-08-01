use std::{env, fs, path::PathBuf, process::ExitCode};

use rascal::semantic;

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
    let source_name = path.to_string_lossy();
    let output = semantic::bind_sources(&[(&source_name, &source)]);

    for diagnostic in &output.diagnostics {
        eprintln!(
            "{}:{}..{}: {}",
            path.display(),
            diagnostic.span.start,
            diagnostic.span.end,
            diagnostic.message
        );
    }
    let Some(file) = output.files.first() else {
        return Ok(false);
    };
    println!(
        "{:?} {}: {} declarations ({} retained as unsupported)",
        file.kind,
        file.pascal_name.as_deref().unwrap_or("<anonymous>"),
        file.declaration_count,
        file.unsupported_declarations,
    );
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
