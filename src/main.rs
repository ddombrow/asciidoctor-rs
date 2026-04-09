use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut format = OutputFormat::Html;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--format" => {
                let Some(value) = args.next() else {
                    eprintln!("usage: asciidoctor-rs [--format html|json|tck-json] <input-file>");
                    return ExitCode::from(2);
                };

                let Some(parsed) = OutputFormat::parse(&value) else {
                    eprintln!("unsupported format: {value}");
                    return ExitCode::from(2);
                };

                format = parsed;
            }
            "--stdin" => {
                return run_with_stdin(format);
            }
            _ => {
                return run_with_path(arg, format);
            }
        }
    }

    eprintln!("usage: asciidoctor-rs [--format html|json|tck-json] [--stdin|<input-file>]");
    ExitCode::from(2)
}

fn run_with_path(path: String, format: OutputFormat) -> ExitCode {
    let input = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let expanded = asciidoctor_rs::preprocess(&input, base_dir);

    let document = asciidoctor_rs::parse_document(&expanded);
    match format {
        OutputFormat::Html => {
            let html = asciidoctor_rs::render_html(&document);
            print!("{html}");
        }
        OutputFormat::Json => {
            let prepared = asciidoctor_rs::prepare_document(&document);
            let json = match asciidoctor_rs::prepared_document_to_json(&prepared) {
                Ok(json) => json,
                Err(error) => {
                    eprintln!("failed to serialize prepared document: {error}");
                    return ExitCode::FAILURE;
                }
            };
            println!("{json}");
        }
        OutputFormat::TckJson => {
            let json = match asciidoctor_rs::render_tck_json(&expanded) {
                Ok(json) => json,
                Err(error) => {
                    eprintln!("failed to serialize TCK document: {error}");
                    return ExitCode::FAILURE;
                }
            };
            println!("{json}");
        }
    }

    ExitCode::SUCCESS
}

fn run_with_stdin(format: OutputFormat) -> ExitCode {
    let mut input = String::new();
    if let Err(error) = io::stdin().read_to_string(&mut input) {
        eprintln!("failed to read stdin: {error}");
        return ExitCode::FAILURE;
    }

    match format {
        OutputFormat::TckJson => match asciidoctor_rs::render_tck_json_from_request(&input) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        OutputFormat::Html | OutputFormat::Json => {
            let document = asciidoctor_rs::parse_document(&input);
            match format {
                OutputFormat::Html => {
                    print!("{}", asciidoctor_rs::render_html(&document));
                }
                OutputFormat::Json => {
                    match asciidoctor_rs::prepared_document_to_json(
                        &asciidoctor_rs::prepare_document(&document),
                    ) {
                        Ok(json) => println!("{json}"),
                        Err(error) => {
                            eprintln!("failed to serialize prepared document: {error}");
                            return ExitCode::FAILURE;
                        }
                    }
                }
                OutputFormat::TckJson => unreachable!(),
            }
            ExitCode::SUCCESS
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Html,
    Json,
    TckJson,
}

impl OutputFormat {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "html" => Some(Self::Html),
            "json" => Some(Self::Json),
            "tck-json" => Some(Self::TckJson),
            _ => None,
        }
    }
}
