use fridge_core::interp::run_program;
use fridge_core::lexer::lex;
use fridge_core::parser::parse;

fn run(src: &str) -> Result<Vec<String>, String> {
    let (tokens, lex_errors) = lex(src);
    assert!(lex_errors.is_empty(), "lex errors: {:?}", lex_errors);
    let (program, parse_errors) = parse(&tokens);
    assert!(parse_errors.is_empty(), "parse errors: {:?}", parse_errors);
    let program = program.expect("program");
    let mut out = Vec::new();
    run_program(&program, &mut out).map(|_| out).map_err(|e| e.code)
}

#[test]
fn hello_world_and_joining() {
    let out = run("remember name as \"Hamsan\"\nshow \"Hello \" joined with name").unwrap();
    assert_eq!(out, vec!["Hello Hamsan"]);
}

#[test]
fn arithmetic_precedence() {
    let out = run("show 2 plus 3 multiplied by 4").unwrap();
    assert_eq!(out, vec!["14"]);
    let out = run("show (2 plus 3) multiplied by 4").unwrap();
    assert_eq!(out, vec!["20"]);
}

#[test]
fn division_semantics() {
    assert_eq!(run("show 6 divided by 2").unwrap(), vec!["3"]);
    assert_eq!(run("show 7 divided by 2").unwrap(), vec!["3.5"]);
    assert_eq!(run("show 7 modulo 2").unwrap(), vec!["1"]);
}

#[test]
fn shortcuts_work_like_long_forms() {
    let out = run("score becomes 10\nif score is greater than 5\n\tprint \"high\"\nelse\n\tprint \"low\"\nend if").unwrap();
    assert_eq!(out, vec!["high"]);
    let out = run("loop 3\n\tshow \"x\"\nend repeat").unwrap();
    assert_eq!(out, vec!["x", "x", "x"]);
}

#[test]
fn if_otherwise_chain() {
    let src = "remember s as 100\nif s is greater than 100\n\tshow \"a\"\notherwise if s is equal to 100\n\tshow \"b\"\notherwise\n\tshow \"c\"\nend if";
    assert_eq!(run(src).unwrap(), vec!["b"]);
}

#[test]
fn repeat_while_break_continue() {
    let src = "remember i as 0\nwhile true\n\tadd 1 to i\n\tif i is less than 3\n\t\tcontinue\n\tend if\n\tbreak\nend while\nshow i";
    assert_eq!(run(src).unwrap(), vec!["3"]);
}

#[test]
fn lists_items_count_first_last() {
    let src = "remember students as a list containing \"Arun\", \"Ravi\", \"Mohan\"\nshow the first item in students\nshow item 2 of students\nshow the number of items in students\nappend \"Deepa\" to students\nshow the last item in students";
    assert_eq!(run(src).unwrap(), vec!["Arun", "Ravi", "3", "Deepa"]);
}

#[test]
fn set_item_of_list() {
    let src = "remember xs as a list containing 1, 2, 3\nset item 2 of xs to 20\nshow xs";
    assert_eq!(run(src).unwrap(), vec!["[1, 20, 3]"]);
}

#[test]
fn range_iteration() {
    let src = "for each i in 1 to 5\n\tshow i\nend for";
    assert_eq!(run(src).unwrap(), vec!["1", "2", "3", "4", "5"]);
}

#[test]
fn functions_params_return_and_call_in_expression() {
    let src = "create a function called total taking price and quantity\n\treturn price multiplied by quantity\nend function\nremember t as call total with 120, 3\nshow t";
    assert_eq!(run(src).unwrap(), vec!["360"]);
}

#[test]
fn recursion_fibonacci() {
    let src = "define fib taking n\n\tif n is less than 2\n\t\treturn n\n\tend if\n\treturn call fib with n minus 1 plus call fib with n minus 2\nend function\nshow call fib with 10";
    assert_eq!(run(src).unwrap(), vec!["55"]);
}

#[test]
fn logic_operators() {
    let src = "remember a as true\nremember b as false\nshow a and not b\nshow a or b\nshow b and a";
    assert_eq!(run(src).unwrap(), vec!["true", "true", "false"]);
}

#[test]
fn events_start_and_end_run_in_order() {
    let src = "when the program starts\n\tshow \"start\"\nend when\nshow \"middle\"\nwhen the program ends\n\tshow \"end\"\nend when";
    assert_eq!(run(src).unwrap(), vec!["start", "middle", "end"]);
}

#[test]
fn runtime_errors_report_codes() {
    assert_eq!(run("show 1 divided by 0").unwrap_err(), "FRG-RUN-001");
    assert_eq!(run("remember xs as a list containing 1\nshow item 5 of xs").unwrap_err(), "FRG-RUN-002");
    assert_eq!(run("show missing_name").unwrap_err(), "FRG-TYPE-002");
    assert_eq!(run("call ghost").unwrap_err(), "FRG-TYPE-008");
}

#[test]
fn gui_statements_report_capability_error() {
    let src = "create a screen called Home\n\tshow \"hi\"\nend screen";
    assert_eq!(run(src).unwrap_err(), "FRG-CAP-001");
}
