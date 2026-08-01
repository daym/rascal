use std::{env, process::ExitCode};

use rascal::{ParseOutput, chumsky_parser, nom_parser};

fn print_output(name: &str, output: &ParseOutput) {
    println!("{name} statements:\n{:#?}", output.statements);
    if !output.diagnostics.is_empty() {
        println!("{name} diagnostics:\n{:#?}", output.diagnostics);
    }
}

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let Some(parser) = arguments.next() else {
        eprintln!("usage: rascal <nom|chumsky|both> <Pascal expression statements>");
        return ExitCode::from(2);
    };
    let source = arguments.collect::<Vec<_>>().join(" ");
    if source.is_empty() {
        eprintln!("rascal: missing Pascal source");
        return ExitCode::from(2);
    }

    match parser.as_str() {
        "nom" => print_output("nom", &nom_parser::parse(&source)),
        "chumsky" => print_output("chumsky", &chumsky_parser::parse(&source)),
        "both" => {
            let nom = nom_parser::parse(&source);
            let chumsky = chumsky_parser::parse(&source);
            print_output("nom", &nom);
            print_output("chumsky", &chumsky);
            if nom.statements != chumsky.statements {
                eprintln!("rascal: parser ASTs differ");
                return ExitCode::FAILURE;
            }
            println!("ASTs agree");
        }
        other => {
            eprintln!("rascal: unknown parser `{other}`; expected nom, chumsky, or both");
            return ExitCode::from(2);
        }
    }

    ExitCode::SUCCESS
}
