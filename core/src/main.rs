use std::env;
use std::fs;
use std::process::ExitCode;

use fridge_core::interp;
use fridge_core::lexer::lex;
use fridge_core::parser::parse;

const VERSION: &str = "FRIDGE 0.1.0-alpha (language spec v0.2.1)";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("version");

    match cmd {
        "version" => { println!("{VERSION}"); ExitCode::SUCCESS }
        "lex" => compile_file(args.get(1), false, false),
        "check" => compile_file(args.get(1), args.iter().any(|a| a == "json"), false),
        "run" => compile_file(args.get(1), false, true),
        other => {
            eprintln!("fridge: unknown command '{other}'");
            eprintln!("available: version | check <file.frg> [json] | run <file.frg> | lex <file.frg>");
            ExitCode::from(2)
        }
    }
}

fn compile_file(path: Option<&String>, json: bool, run: bool) -> ExitCode {
    let Some(path) = path else {
        eprintln!("usage: fridge <check|run|lex> <file.frg>");
        return ExitCode::from(64);
    };
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: FRG-IO-001\nFile: {path}\nLine: 0\n\nCannot read file: {e}\n");
            return ExitCode::FAILURE;
        }
    };

    let (tokens, lex_errors) = lex(&src);
    let (program, parse_errors) = if lex_errors.is_empty() {
        parse(&tokens)
    } else {
        (None, Vec::new())
    };

    if json {
        println!("{{\"diagnostics\":[");
        let mut first = true;
        for e in lex_errors.iter().map(|e| (e.line, e.col, e.code.clone(), e.message.clone()))
            .chain(parse_errors.iter().map(|e| (e.line, e.col, e.code.clone(), e.message.clone())))
        {
            if !first { println!(","); }
            first = false;
            print!(
                "{{\"file\":\"{}\",\"line\":{},\"column\":{},\"severity\":\"error\",\"code\":\"{}\",\"message\":\"{}\",\"explanation\":\"\",\"suggestion\":\"\",\"fixes\":[]}}",
                escape(path), e.0, e.1, e.2, escape(&e.3));
        }
        println!("]}}");
        return if first { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }

    for e in &lex_errors {
        println!("ERROR: {}\nFile: {}\nLine: {}\n\n{}\n", e.code, path, e.line, e.message);
    }
    for e in &parse_errors {
        println!("ERROR: {}\nFile: {}\nLine: {}\n\n{}\n", e.code, path, e.line, e.message);
    }
    if !lex_errors.is_empty() || !parse_errors.is_empty() {
        return ExitCode::FAILURE;
    }

    if !run {
        println!("{}: OK — no problems found.", path);
        return ExitCode::SUCCESS;
    }

    let program = program.expect("program must exist when there are no errors");
    let mut out: Vec<String> = Vec::new();
    match interp::run_program(&program, &mut out) {
        Ok(notices) => {
            for line in &out { println!("{line}"); }
            for n in &notices { eprintln!("notice: {n}"); }
            ExitCode::SUCCESS
        }
        Err(e) => {
            for line in &out { println!("{line}"); }
            for n in interp_notices_placeholder() { eprintln!("notice: {n}"); }
            eprintln!("ERROR: {}\nFile: {}\nLine: {}\n\n{}\n", e.code, path, e.line, e.message);
            ExitCode::from(2)
        }
    }
}

fn interp_notices_placeholder() -> Vec<String> { Vec::new() }

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}
