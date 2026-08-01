use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use rascal::{TokenKind, lex, pascal_parser};

fn collect_pascal_files(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(root).map_err(|error| format!("cannot read {}: {error}", root.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_pascal_files(&path, output)?;
        } else if path.extension().is_some_and(|extension| extension == "pp") {
            output.push(path);
        }
    }
    Ok(())
}

fn line_and_column(source: &str, byte: usize) -> (usize, usize) {
    let prefix = &source[..byte.min(source.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.len() + 1, |(_, tail)| tail.len() + 1);
    (line, column)
}

fn run() -> Result<bool, String> {
    let mut root = PathBuf::from("tests/pascal/generated");
    let mut inventory = false;
    let mut parse_files = false;
    for argument in env::args().skip(1) {
        if argument == "--inventory" {
            inventory = true;
        } else if argument == "--parse" {
            parse_files = true;
        } else {
            root = PathBuf::from(argument);
        }
    }
    let mut files = Vec::new();
    collect_pascal_files(&root, &mut files)?;
    files.sort();

    let mut token_count = 0usize;
    let mut bad_files = 0usize;
    let mut diagnostic_count = 0usize;
    let mut shown = 0usize;
    let mut spellings = BTreeMap::<String, usize>::new();
    let mut identifiers = BTreeMap::<String, usize>::new();
    let mut parsed_files = 0usize;
    let mut parse_diagnostics = 0usize;
    let mut parse_bad_files = 0usize;
    let mut parse_shown = 0usize;
    for path in &files {
        let source = fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let output = lex(&source);
        token_count += output.tokens.len();
        for token in &output.tokens {
            if let TokenKind::Identifier(name) = &token.kind {
                *identifiers.entry(name.clone()).or_default() += 1;
            }
        }
        if !output.diagnostics.is_empty() {
            bad_files += 1;
        }
        diagnostic_count += output.diagnostics.len();
        for diagnostic in &output.diagnostics {
            let spelling = source
                .get(diagnostic.span.clone())
                .unwrap_or("<invalid span>")
                .escape_debug();
            *spellings.entry(spelling.to_string()).or_default() += 1;
            if shown >= 40 {
                continue;
            }
            let (line, column) = line_and_column(&source, diagnostic.span.start);
            eprintln!(
                "{}:{line}:{column}: {}: `{spelling}`",
                path.display(),
                diagnostic.message
            );
            shown += 1;
        }
        if parse_files {
            let parsed = pascal_parser::parse(&source);
            if parsed.file.is_some() {
                parsed_files += 1;
            }
            if !parsed.diagnostics.is_empty() {
                parse_bad_files += 1;
            }
            parse_diagnostics += parsed.diagnostics.len();
            for diagnostic in parsed.diagnostics {
                if parse_shown >= 40 {
                    continue;
                }
                let (line, column) = line_and_column(&source, diagnostic.span.start);
                eprintln!("{}:{line}:{column}: {}", path.display(), diagnostic.message);
                parse_shown += 1;
            }
        }
    }

    println!(
        "{} files, {} tokens, {} lexer diagnostics in {} files",
        files.len(),
        token_count,
        diagnostic_count,
        bad_files
    );
    let mut spellings = spellings.into_iter().collect::<Vec<_>>();
    spellings.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (spelling, count) in spellings {
        println!("{count:>6}  `{spelling}`");
    }
    if inventory {
        let mut identifiers = identifiers.into_iter().collect::<Vec<_>>();
        identifiers.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        for (identifier, count) in identifiers.into_iter().take(250) {
            println!("{count:>6}  {identifier}");
        }
    }
    if parse_files {
        println!(
            "{parsed_files} files produced CSTs, {parse_diagnostics} structural diagnostics in {parse_bad_files} files"
        );
    }
    Ok(diagnostic_count == 0 && (!parse_files || parsed_files == files.len()))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("check_pascal_corpus: {error}");
            ExitCode::FAILURE
        }
    }
}
