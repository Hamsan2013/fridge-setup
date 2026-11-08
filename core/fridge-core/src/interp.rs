use std::collections::HashMap;
use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeError { pub code: String, pub message: String, pub line: usize }

#[derive(Debug, Clone, PartialEq)]
pub enum Value { Int(i64), Float(f64), Str(String), Bool(bool), Nothing, List(Vec<Value>), Range(i64, i64) }

type Scope = HashMap<String, Value>;
enum Flow { Next, Break, Continue, Return(Option<Value>) }

pub fn run_program(program: &Program, out: &mut Vec<String>) -> Result<Vec<String>, RuntimeError> {
    let mut interp = Interpreter { globals: Scope::new(), functions: HashMap::new(), depth: 0, notices: Vec::new() };
    let mut locals: Vec<Scope> = Vec::new();
    let mut end_bodies: Vec<Vec<Stmt>> = Vec::new();
    let mut stopped = false;
    for stmt in &program.stmts {
        match stmt {
            Stmt::Function { name, params, body, .. } => { interp.functions.insert(name.clone(), (params.clone(), body.clone())); }
            Stmt::When { event, body, .. } => match event {
                Event::ProgramStarts => { interp.exec_block(body, &mut locals, out)?; }
                Event::ProgramEnds => end_bodies.push(body.clone()),
                other => interp.notices.push(format!("event '{}' needs the GUI/game runtime (Phase 5+) and will not fire in console mode", event_name(other))),
            },
            other => {
                match interp.exec_stmt(other, &mut locals, out)? {
                    Flow::Next => {}
                    Flow::Return(_) => { stopped = true; }
                    Flow::Break => return Err(RuntimeError { code: "FRG-RUN-005".into(), line: stmt_line(other), message: "'break' is only allowed inside a loop.".into() }),
                    Flow::Continue => return Err(RuntimeError { code: "FRG-RUN-005".into(), line: stmt_line(other), message: "'continue' is only allowed inside a loop.".into() }),
                }
                if stopped { break; }
            }
        }
    }
    for body in &end_bodies { interp.exec_block(body, &mut locals, out)?; }
    Ok(interp.notices)
}

fn event_name(e: &Event) -> String {
    match e { Event::ProgramStarts=>"program starts".into(), Event::ProgramEnds=>"program ends".into(), Event::EachFrame=>"each frame".into(), Event::Pressed(_)=>"pressed".into(), Event::Released(_)=>"released".into(), Event::Touched(_)=>"touched".into(), Event::Collides(_,_)=>"collides".into() }
}

fn stmt_line(s: &Stmt) -> usize {
    match s { Stmt::Remember{line,..}|Stmt::Set{line,..}|Stmt::Update{line,..}|Stmt::Show{line,..}|Stmt::Call{line,..}|Stmt::Return{line,..}|Stmt::Break{line}|Stmt::Continue{line}|Stmt::If{line,..}|Stmt::Repeat{line,..}|Stmt::While{line,..}|Stmt::ForEach{line,..}|Stmt::Function{line,..}|Stmt::When{line,..}|Stmt::Use{line,..}|Stmt::Gui{line,..}=>*line }
}

struct Interpreter { globals: Scope, functions: HashMap<String,(Vec<String>,Vec<Stmt>)>, depth: usize, notices: Vec<String> }

impl Interpreter {
    fn err(&self, code: &str, msg: &str, line: usize) -> RuntimeError { RuntimeError { code: code.into(), message: msg.into(), line } }
    fn lookup(&self, locals: &[Scope], name: &str) -> Option<Value> {
        for s in locals.iter().rev() { if let Some(v) = s.get(name) { return Some(v.clone()); } }
        self.globals.get(name).cloned()
    }
    fn declare(&mut self, locals: &mut Vec<Scope>, name: &str, value: Value, line: usize) -> Result<(), RuntimeError> {
        if let Some(top) = locals.last_mut() {
            if top.contains_key(name) { return Err(self.err("FRG-TYPE-007", &format!("'{name}' is already remembered in this scope."), line)); }
            top.insert(name.to_string(), value);
        } else {
            if self.globals.contains_key(name) { return Err(self.err("FRG-TYPE-007", &format!("'{name}' is already remembered."), line)); }
            self.globals.insert(name.to_string(), value);
        }
        Ok(())
    }
    fn update(&mut self, locals: &mut Vec<Scope>, name: &str, value: Value, line: usize) -> Result<(), RuntimeError> {
        for s in locals.iter_mut().rev() { if s.contains_key(name) { s.insert(name.to_string(), value); return Ok(()); } }
        if self.globals.contains_key(name) { self.globals.insert(name.to_string(), value); return Ok(()); }
        Err(self.err("FRG-TYPE-002", &format!("'{name}' was never remembered. Add: remember {name} as ..."), line))
    }
    fn store(&mut self, locals: &mut Vec<Scope>, place: &Expr, value: Value) -> Result<(), RuntimeError> {
        match place {
            Expr::Var { name, line } => self.update(locals, name, value, *line),
            Expr::Index { index, list, line } => {
                let container = self.eval(list, locals)?;
                let Value::List(mut items) = container else { return Err(self.err("FRG-TYPE-003", "Only items of a list can be changed this way.", *line)); };
                let i = self.eval_to_index(index, &items, locals, *line)?;
                items[i] = value;
                self.store(locals, list, Value::List(items))
            }
            Expr::Field { line, .. } => Err(self.err("FRG-TYPE-010", "Properties and structures arrive in Phase 3.", *line)),
            _ => Err(self.err("FRG-INT-001", "Invalid assignment target.", 0)),
        }
    }
    fn eval_to_index(&mut self, index: &Expr, items: &[Value], locals: &[Scope], line: usize) -> Result<usize, RuntimeError> {
        let v = self.eval(index, locals)?;
        let Value::Int(i) = v else { return Err(self.err("FRG-TYPE-003", "List positions must be whole numbers.", line)); };
        if i < 1 || i as usize > items.len() { return Err(self.err("FRG-RUN-002", &format!("Position {i} does not exist. FRIDGE counts from 1; this list has {} item(s).", items.len()), line)); }
        Ok((i - 1) as usize)
    }
    fn exec_block(&mut self, body: &[Stmt], locals: &mut Vec<Scope>, out: &mut Vec<String>) -> Result<(), RuntimeError> {
        match self.exec_stmts(body, locals, out)? { Flow::Next|Flow::Return(_)=>Ok(()), Flow::Break=>Err(self.err("FRG-RUN-005","'break' is only allowed inside a loop.",0)), Flow::Continue=>Err(self.err("FRG-RUN-005","'continue' is only allowed inside a loop.",0)) }
    }
    fn exec_stmts(&mut self, stmts: &[Stmt], locals: &mut Vec<Scope>, out: &mut Vec<String>) -> Result<Flow, RuntimeError> {
        for s in stmts { match self.exec_stmt(s, locals, out)? { Flow::Next=>{} other=>return Ok(other) } }
        Ok(Flow::Next)
    }
    fn exec_stmt(&mut self, s: &Stmt, locals: &mut Vec<Scope>, out: &mut Vec<String>) -> Result<Flow, RuntimeError> {
        match s {
            Stmt::Remember { name, value, line } => { let v = self.eval(value, locals)?; self.declare(locals, name, v, *line)?; }
            Stmt::Set { target, value, line } => { let v = self.eval(value, locals)?; self.store(locals, target, v).map_err(|mut e|{ if e.line==0{e.line=*line;} e })?; }
            Stmt::Update { op, target, value, line } => {
                let rhs = self.eval(value, locals)?; let current = self.eval(target, locals)?;
                let updated = match op {
                    UpdateOp::AddTo => self.arith(BinOp::Add, current, rhs, *line)?,
                    UpdateOp::SubtractFrom => self.arith(BinOp::Sub, current, rhs, *line)?,
                    UpdateOp::MultiplyBy => self.arith(BinOp::Mul, current, rhs, *line)?,
                    UpdateOp::DivideBy => self.arith(BinOp::Div, current, rhs, *line)?,
                    UpdateOp::AppendTo => match current { Value::List(mut items)=>{items.push(rhs);Value::List(items)} Value::Str(t)=>Value::Str(t+&display(&rhs)), _=>return Err(self.err("FRG-TYPE-003","'append' works on lists and text.",*line)) },
                };
                self.store(locals, target, updated)?;
            }
            Stmt::Show { value, .. } => { let v = self.eval(value, locals)?; out.push(display(&v)); }
            Stmt::Call { name, args, line } => { self.call(name, args, locals, out, *line)?; }
            Stmt::Return { value, .. } => { let v = match value { Some(e)=>Some(self.eval(e,locals)?), None=>None }; return Ok(Flow::Return(v)); }
            Stmt::Break { .. } => return Ok(Flow::Break),
            Stmt::Continue { .. } => return Ok(Flow::Continue),
            Stmt::If { arms, otherwise, .. } => {
                for (cond, body) in arms { let v = self.eval(cond, locals)?; let Value::Bool(b) = v else { return Err(self.err("FRG-TYPE-006","'if' needs a true/false condition.",stmt_line(s))); }; if b { return self.exec_stmts(body, locals, out); } }
                if let Some(body) = otherwise { return self.exec_stmts(body, locals, out); }
            }
            Stmt::Repeat { count, body, line } => {
                let times = match count { None=>i64::MAX, Some(e)=>{ let v=self.eval(e,locals)?; let Value::Int(n)=v else { return Err(self.err("FRG-TYPE-011","'repeat' needs a whole number of times.",*line)); }; if n<0 { return Err(self.err("FRG-RUN-006","'repeat' needs zero or more times.",*line)); } n } };
                for _ in 0..times { locals.push(Scope::new()); let flow=self.exec_stmts(body,locals,out); locals.pop(); match flow? { Flow::Break=>break, Flow::Return(v)=>return Ok(Flow::Return(v)), _=>{} } }
            }
            Stmt::While { cond, body, .. } => loop {
                let v = self.eval(cond, locals)?; let Value::Bool(b) = v else { return Err(self.err("FRG-TYPE-006","'while' needs a true/false condition.",stmt_line(s))); };
                if !b { break; }
                locals.push(Scope::new()); let flow=self.exec_stmts(body,locals,out); locals.pop();
                match flow? { Flow::Break=>break, Flow::Return(v)=>return Ok(Flow::Return(v)), _=>{} }
            },
            Stmt::ForEach { var, iter, body, line } => {
                let v = self.eval(iter, locals)?;
                let items: Vec<Value> = match v { Value::List(items)=>items, Value::Range(a,b)=>(a..=b).map(Value::Int).collect(), _=>return Err(self.err("FRG-TYPE-009","'for each' walks through lists or ranges like 1 to 10.",*line)) };
                for item in items { locals.push(Scope::new()); locals.last_mut().unwrap().insert(var.clone(),item); let flow=self.exec_stmts(body,locals,out); locals.pop(); match flow? { Flow::Break=>break, Flow::Return(v)=>return Ok(Flow::Return(v)), _=>{} } }
            }
            Stmt::Function { name, params, body, .. } => { self.functions.insert(name.clone(), (params.clone(), body.clone())); }
            Stmt::When { .. } => {}
            Stmt::Use { module, .. } => { self.notices.push(format!("module '{module}' is recognized but ships no features in this alpha build (standard library is Phase 4)")); }
            Stmt::Gui { line, .. } => { return Err(self.err("FRG-CAP-001","This program uses screens/controls. The GUI runtime arrives in Phase 5; this console build cannot draw it yet.",*line)); }
        }
        Ok(Flow::Next)
    }
    fn call(&mut self, name: &str, args: &[Expr], locals: &mut Vec<Scope>, out: &mut Vec<String>, line: usize) -> Result<Option<Value>, RuntimeError> {
        let (params, body) = self.functions.get(name).cloned().ok_or_else(|| self.err("FRG-TYPE-008", &format!("There is no function called '{name}'."), line))?;
        if params.len() != args.len() { return Err(self.err("FRG-TYPE-004", &format!("'{name}' expects {} input(s) but received {}.", params.len(), args.len()), line)); }
        self.depth += 1;
        if self.depth > 512 { return Err(self.err("FRG-RUN-010", "Too many nested function calls -- possible endless recursion.", line)); }
        let mut values = Vec::new();
        for a in args { values.push(self.eval(a, locals)?); }
        locals.push(Scope::new());
        for (p, v) in params.iter().zip(values) { locals.last_mut().unwrap().insert(p.clone(), v); }
        let flow = self.exec_stmts(&body, locals, out)?;
        locals.pop();
        self.depth -= 1;
        match flow { Flow::Next|Flow::Break|Flow::Continue=>Ok(None), Flow::Return(v)=>Ok(v) }
    }
    fn eval(&mut self, e: &Expr, locals: &[Scope]) -> Result<Value, RuntimeError> {
        match e {
            Expr::Int(v) => Ok(Value::Int(*v)),
            Expr::Float(v) => Ok(Value::Float(*v)),
            Expr::Str(v) => Ok(Value::Str(v.clone())),
            Expr::Bool(v) => Ok(Value::Bool(*v)),
            Expr::Nothing => Ok(Value::Nothing),
            Expr::Var { name, line } => self.lookup(locals, name).ok_or_else(|| self.err("FRG-TYPE-002", &format!("'{name}' was never remembered. Add: remember {name} as ..."), *line)),
            Expr::List { items, .. } => { let mut values = Vec::new(); for i in items { values.push(self.eval(i, locals)?); } Ok(Value::List(values)) }
            Expr::Range { from, to, line } => { let a=self.eval(from,locals)?; let b=self.eval(to,locals)?; let (Value::Int(x),Value::Int(y))=(a,b) else { return Err(self.err("FRG-TYPE-003","Ranges need whole numbers, like 1 to 10.",*line)); }; Ok(Value::Range(x,y)) }
            Expr::Concat { left, right } => { let l=self.eval(left,locals)?; let r=self.eval(right,locals)?; Ok(Value::Str(display(&l)+&display(&r))) }
            Expr::Neg(inner) => match self.eval(inner, locals)? { Value::Int(v)=>Ok(Value::Int(-v)), Value::Float(v)=>Ok(Value::Float(-v)), _=>Err(self.err("FRG-TYPE-003","'-' needs a number.",0)) },
            Expr::Not(inner) => match self.eval(inner, locals)? { Value::Bool(b)=>Ok(Value::Bool(!b)), _=>Err(self.err("FRG-TYPE-005","'not' needs true or false.",0)) },
            Expr::Bin { op, left, right, line } => { let l=self.eval(left,locals)?; let r=self.eval(right,locals)?; self.arith(*op,l,r,*line) }
            Expr::Cmp { op, negated, left, right, line } => {
                let l=self.eval(left,locals)?; let r=self.eval(right,locals)?;
                let result = match op { CmpOp::Eq=>value_eq(&l,&r), _=>{ let (Some(x),Some(y))=(as_f64(&l),as_f64(&r)) else { return Err(self.err("FRG-TYPE-003","'greater than' / 'less than' compare numbers.",*line)); }; match op { CmpOp::Gt=>x>y, CmpOp::Lt=>x<y, CmpOp::Ge=>x>=y, CmpOp::Le=>x<=y, CmpOp::Eq=>unreachable!() } } };
                Ok(Value::Bool(result ^ negated))
            }
            Expr::And { left, right } => { let l=self.eval(left,locals)?; let Value::Bool(b)=l else { return Err(self.err("FRG-TYPE-005","'and' needs true/false on both sides.",0)); }; if !b { return Ok(Value::Bool(false)); } let r=self.eval(right,locals)?; let Value::Bool(b2)=r else { return Err(self.err("FRG-TYPE-005","'and' needs true/false on both sides.",0)); }; Ok(Value::Bool(b2)) }
            Expr::Or { left, right } => { let l=self.eval(left,locals)?; let Value::Bool(b)=l else { return Err(self.err("FRG-TYPE-005","'or' needs true/false on both sides.",0)); }; if b { return Ok(Value::Bool(true)); } let r=self.eval(right,locals)?; let Value::Bool(b2)=r else { return Err(self.err("FRG-TYPE-005","'or' needs true/false on both sides.",0)); }; Ok(Value::Bool(b2)) }
            Expr::Call { name, args, line } => { let v = self.call(name, args, locals, out_placeholder(), *line)?; Ok(v.unwrap_or(Value::Nothing)) }
            Expr::Index { index, list, line } => { let container=self.eval(list,locals)?; let Value::List(items)=container else { return Err(self.err("FRG-TYPE-003","'item ... of' works on lists.",*line)); }; let i=self.eval_to_index(index,&items,locals,*line)?; Ok(items[i].clone()) }
            Expr::First { list, line } => { let Value::List(items)=self.eval(list,locals)? else { return Err(self.err("FRG-TYPE-003","'first item' works on lists.",*line)); }; items.first().cloned().ok_or_else(|| self.err("FRG-RUN-003","The list is empty -- there is no first item.",*line)) }
            Expr::Last { list, line } => { let Value::List(items)=self.eval(list,locals)? else { return Err(self.err("FRG-TYPE-003","'last item' works on lists.",*line)); }; items.last().cloned().ok_or_else(|| self.err("FRG-RUN-003","The list is empty -- there is no last item.",*line)) }
            Expr::Count { list, line } => { let Value::List(items)=self.eval(list,locals)? else { return Err(self.err("FRG-TYPE-003","'number of items' works on lists.",*line)); }; Ok(Value::Int(items.len() as i64)) }
            Expr::Field { line, .. } => Err(self.err("FRG-TYPE-010", "Properties and structures arrive in Phase 3.", *line)),
        }
    }
    fn arith(&self, op: BinOp, a: Value, b: Value, line: usize) -> Result<Value, RuntimeError> {
        match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => match op {
                BinOp::Add => Ok(Value::Int(x.wrapping_add(*y))),
                BinOp::Sub => Ok(Value::Int(x.wrapping_sub(*y))),
                BinOp::Mul => Ok(Value::Int(x.wrapping_mul(*y))),
                BinOp::Div => { if *y==0 { return Err(self.err("FRG-RUN-001","Dividing by zero is not allowed.",line)); } if x%y==0 { Ok(Value::Int(x/y)) } else { Ok(Value::Float(*x as f64 / *y as f64)) } }
                BinOp::Mod => { if *y==0 { return Err(self.err("FRG-RUN-001","Dividing by zero is not allowed.",line)); } Ok(Value::Int(x%y)) }
            },
            _ => {
                let (Some(x),Some(y))=(as_f64(&a),as_f64(&b)) else { return Err(self.err("FRG-TYPE-001", &format!("Math needs numbers, got: {} and {}.", display(&a), display(&b)), line)); };
                match op {
                    BinOp::Add=>Ok(Value::Float(x+y)), BinOp::Sub=>Ok(Value::Float(x-y)), BinOp::Mul=>Ok(Value::Float(x*y)),
                    BinOp::Div=>{ if y==0.0 { return Err(self.err("FRG-RUN-001","Dividing by zero is not allowed.",line)); } Ok(Value::Float(x/y)) }
                    BinOp::Mod=>{ if y==0.0 { return Err(self.err("FRG-RUN-001","Dividing by zero is not allowed.",line)); } Ok(Value::Float(x%y)) }
                }
            }
        }
    }
}

fn out_placeholder() -> &'static mut Vec<String> {
    static mut EMPTY: Vec<String> = Vec::new();
    unsafe { &mut EMPTY }
}

fn as_f64(v: &Value) -> Option<f64> { match v { Value::Int(i)=>Some(*i as f64), Value::Float(f)=>Some(*f), _=>None } }

fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x),Value::Int(y))=>x==y,
        (Value::Float(x),Value::Float(y))=>x==y,
        (Value::Int(x),Value::Float(y))|(Value::Float(y),Value::Int(x))=>(*x as f64)==*y,
        (Value::Str(x),Value::Str(y))=>x==y,
        (Value::Bool(x),Value::Bool(y))=>x==y,
        (Value::Nothing,Value::Nothing)=>true,
        (Value::List(x),Value::List(y))=>x.len()==y.len() && x.iter().zip(y).all(|(p,q)|value_eq(p,q)),
        (Value::Range(a1,a2),Value::Range(b1,b2))=>a1==b1 && a2==b2,
        _=>false,
    }
}

pub fn display(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => { if f.fract()==0.0 && f.abs()<9.0e15 { format!("{}",*f as i64) } else { format!("{f}") } }
        Value::Str(s) => s.clone(),
        Value::Bool(b) => { if *b { "true".into() } else { "false".into() } }
        Value::Nothing => "nothing".into(),
        Value::List(items) => { let parts: Vec<String> = items.iter().map(display).collect(); format!("[{}]", parts.join(", ")) }
        Value::Range(a, b) => format!("{a} to {b}"),
    }
                                                               }
