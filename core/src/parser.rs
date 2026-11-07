  //! FRIDGE recursive-descent parser — language spec v0.2.1.
//! One precise diagnostic per problem; multi-error recovery is a planned improvement.

use crate::ast::*;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub col: usize,
    pub code: String,
    pub message: String,
}

pub fn parse(tokens: &[Token]) -> (Option<Program>, Vec<ParseError>) {
    let mut p = Parser { tokens, pos: 0 };
    let mut stmts = Vec::new();
    loop {
        p.skip_newlines();
        if p.is(TokenKind::Eof) {
            return (Some(Program { stmts }), Vec::new());
        }
        match p.parse_statement() {
            Ok(s) => stmts.push(s),
            Err(e) => return (None, vec![e]),
        }
        if !(p.is(TokenKind::Newline) || p.is(TokenKind::Eof)) {
            return (None, vec![p.err_here("FRG-PARSE-003",
                "Each instruction must be on its own line.")]);
        }
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

enum BlockEnd { Kw(TokenKind), If }
enum BlockStop { End, Otherwise }

impl<'a> Parser<'a> {
    // ---------- token helpers ----------

    fn peek(&self) -> &TokenKind {
        self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }
    fn peek_token(&self) -> &Token {
        static EOF: std::sync::OnceLock<Token> =
            std::sync::OnceLock::new();
        self.tokens.get(self.pos).unwrap_or_else(|| {
            EOF.get_or_init(|| Token { kind: TokenKind::Eof, line: 0, col: 0 })
        })
    }
    fn advance(&mut self) -> TokenKind {
        let k = self.peek().clone();
        if self.pos < self.tokens.len() { self.pos += 1; }
        k
    }
    fn is(&self, k: TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(&k)
    }
    fn is_at(&self, n: usize, k: TokenKind) -> bool {
        let d = self.tokens.get(self.pos + n).map(|t| &t.kind).unwrap_or(&TokenKind::Eof);
        std::mem::discriminant(d) == std::mem::discriminant(&k)
    }
    fn ident_at(&self, n: usize) -> Option<String> {
        match self.tokens.get(self.pos + n).map(|t| &t.kind) {
            Some(TokenKind::Ident(s)) => Some(s.clone()),
            _ => None,
        }
    }
    fn skip_newlines(&mut self) {
        while self.is(TokenKind::Newline) { self.pos += 1; }
    }
    fn err_here(&self, code: &str, message: &str) -> ParseError {
        let t = self.peek_token();
        ParseError { line: t.line, col: t.col, code: code.to_string(), message: message.to_string() }
    }
    fn expect(&mut self, k: TokenKind) -> Result<(), ParseError> {
        if self.is(k.clone()) { self.advance(); Ok(()) }
        else {
            Err(self.err_here("FRG-PARSE-001",
                &format!("Expected '{}', found '{}'.", describe(&k), describe(self.peek()))))
        }
    }
    fn expect_ident(&mut self) -> Result<(String, usize), ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(name) => { let line = self.peek_token().line; self.advance(); Ok((name, line)) }
            _ => Err(self.err_here("FRG-PARSE-001",
                &format!("Expected a name, found '{}'.", describe(self.peek())))),
        }
    }
    /// Names may also be quoted ("My Game") or use reserved display words like `game`.
    fn expect_name_value(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(s) => { self.advance(); Ok(s) }
            TokenKind::Str(s) => { self.advance(); Ok(s) }
            TokenKind::Game => { self.advance(); Ok("game".into()) }
            TokenKind::Screen => { self.advance(); Ok("screen".into()) }
            _ => Err(self.err_here("FRG-PARSE-001",
                &format!("Expected a name, found '{}'.", describe(self.peek())))),
        }
    }

    // ---------- statements ----------

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        let line = self.peek_token().line;
        match self.peek().clone() {
            TokenKind::Remember => {
                self.advance();
                let (name, _) = self.expect_ident()?;
                self.expect(TokenKind::As)?;
                let value = self.parse_expr(true)?;
                Ok(Stmt::Remember { name, value, line })
            }
            TokenKind::Set => {
                self.advance();
                let target = self.parse_assignable()?;
                self.expect(TokenKind::To)?;
                let value = self.parse_expr(true)?;
                Ok(Stmt::Set { target, value, line })
            }
            TokenKind::Ident(_) if self.is_at(1, TokenKind::Becomes) => {
                let (name, _) = self.expect_ident()?;
                self.advance(); // becomes
                let value = self.parse_expr(true)?;
                Ok(Stmt::Set { target: Expr::Var { name, line }, value, line })
            }
            TokenKind::Show => {
                self.advance();
                let value = self.parse_expr(true)?;
                Ok(Stmt::Show { value, line })
            }
            TokenKind::Show2 => unreachable!(),
            TokenKind::Show_ => unreachable!(),
            TokenKind::Add => self.parse_add_statement(line),
            TokenKind::Subtract => {
                self.advance();
                let value = self.parse_expr(false)?;
                self.expect(TokenKind::From)?;
                let target = self.parse_assignable()?;
                Ok(Stmt::Update { op: UpdateOp::SubtractFrom, target, value, line })
            }
            TokenKind::Append => {
                self.advance();
                let value = self.parse_expr(false)?;
                self.expect(TokenKind::To)?;
                let target = self.parse_assignable()?;
                Ok(Stmt::Update { op: UpdateOp::AppendTo, target, value, line })
            }
            TokenKind::Multiply => {
                self.advance();
                let target = self.parse_assignable()?;
                self.expect(TokenKind::By)?;
                let value = self.parse_expr(true)?;
                Ok(Stmt::Update { op: UpdateOp::MultiplyBy, target, value, line })
            }
            TokenKind::Divide => {
                self.advance();
                let target = self.parse_assignable()?;
                self.expect(TokenKind::By)?;
                let value = self.parse_expr(true)?;
                Ok(Stmt::Update { op: UpdateOp::DivideBy, target, value, line })
            }
            TokenKind::Call => {
                self.advance();
                let (name, _) = self.expect_ident()?;
                let args = self.parse_args()?;
                Ok(Stmt::Call { name, args, line })
            }
            TokenKind::Return => {
                self.advance();
                let value = if self.is(TokenKind::Newline) || self.is(TokenKind::Eof) {
                    None
                } else {
                    Some(self.parse_expr(true)?)
                };
                Ok(Stmt::Return { value, line })
            }
            TokenKind::Break => { self.advance(); Ok(Stmt::Break { line }) }
            TokenKind::Continue => { self.advance(); Ok(Stmt::Continue { line }) }
            TokenKind::If => self.parse_if(line),
            TokenKind::Repeat => {
                self.advance();
                let count = if self.is(TokenKind::Forever) {
                    self.advance();
                    None
                } else {
                    let e = self.parse_expr(true)?;
                    if self.is(TokenKind::Times) { self.advance(); }
                    Some(e)
                };
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::Repeat))?;
                Ok(Stmt::Repeat { count, body, line })
            }
            TokenKind::While => {
                self.advance();
                let cond = self.parse_expr(true)?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::While))?;
                Ok(Stmt::While { cond, body, line })
            }
            TokenKind::For => {
                self.advance();
                self.expect(TokenKind::Each)?;
                let (var, _) = self.expect_ident()?;
                self.expect(TokenKind::In)?;
                let iter = self.parse_expr(true)?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::For))?;
                Ok(Stmt::ForEach { var, iter, body, line })
            }
            TokenKind::Create => self.parse_create(line),
            TokenKind::Define => {
                self.advance();
                let (name, _) = self.expect_ident()?;
                let params = self.parse_params()?;
                self.parse_returning()?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::Function))?;
                Ok(Stmt::Function { name, params, body, line })
            }
            TokenKind::When => {
                self.advance();
                let event = self.parse_event()?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::When))?;
                Ok(Stmt::When { event, body, line })
            }
            TokenKind::Use => {
                self.advance();
                let (module, _) = self.expect_ident()?;
                Ok(Stmt::Use { module, line })
            }
            TokenKind::Place => {
                self.advance();
                let name = self.expect_name_value()?;
                let mut at = Vec::new();
                let mut size = Vec::new();
                if self.is(TokenKind::At) {
                    self.advance();
                    loop {
                        match self.ident_at(0).as_deref() {
                            Some("x") | Some("y") | Some("z") => {
                                let (axis, _) = self.expect_ident()?;
                                let v = self.parse_expr(true)?;
                                at.push((axis, v));
                            }
                            _ => break,
                        }
                    }
                    if at.is_empty() {
                        return Err(self.err_here("FRG-PARSE-001",
                            "'at' needs coordinates, for example: at x 10 y 20"));
                    }
                }
                if self.ident_at(0).as_deref() == Some("size") {
                    self.advance();
                    loop {
                        match self.ident_at(0).as_deref() {
                            Some("width") | Some("height") | Some("depth") => {
                                let (dim, _) = self.expect_ident()?;
                                let v = self.parse_expr(true)?;
                                size.push((dim, v));
                            }
                            _ => break,
                        }
                    }
                }
                Ok(Stmt::Gui { kind: GuiStmt::Place { name, at, size }, line })
            }
            TokenKind::Open => {
                self.advance();
                let name = self.expect_name_value()?;
                Ok(Stmt::Gui { kind: GuiStmt::Open { name }, line })
            }
            _ => Err(self.err_here("FRG-PARSE-001",
                &format!("'{}' does not start any FRIDGE instruction.", describe(self.peek())))),
        }
    }

    fn parse_add_statement(&mut self, line: usize) -> Result<Stmt, ParseError> {
        // GUI form:   add a button called start showing "Start" ...
        // arithmetic: add 10 to score
        if self.is_at(1, TokenKind::Ident(String::new())) && self.is_at(2, TokenKind::Called) {
            self.advance(); // add
            let (kind, _) = self.expect_ident()?;
            self.expect(TokenKind::Called)?;
            let name = self.expect_name_value()?;
            let showing = if self.is(TokenKind::Showing) {
                self.advance();
                Some(self.parse_expr(true)?)
            } else { None };
            let mut options = Vec::new();
            if self.is(TokenKind::With) {
                self.advance();
                loop {
                    let (key, _) = self.expect_ident()?;
                    let val = self.parse_expr(true)?;
                    options.push((key, val));
                    if self.is(TokenKind::Comma) { self.advance(); } else { break; }
                }
            }
            return Ok(Stmt::Gui { kind: GuiStmt::AddControl { kind, name, showing, options }, line });
        }
        self.advance(); // add
        let value = self.parse_expr(false)?;
        self.expect(TokenKind::To)?;
        let target = self.parse_assignable()?;
        Ok(Stmt::Update { op: UpdateOp::AddTo, target, value, line })
    }

    fn parse_if(&mut self, line: usize) -> Result<Stmt, ParseError> {
        self.advance(); // if
        let mut arms = Vec::new();
        let mut otherwise = None;
        loop {
            let cond = self.parse_expr(true)?;
            let (body, stop) = self.parse_block(BlockEnd::If)?;
            arms.push((cond, body));
            match stop {
                BlockStop::End => break,
                BlockStop::Otherwise => {
                    self.advance(); // otherwise
                    if self.is(TokenKind::If) { self.advance(); continue; }
                    if otherwise.is_some() {
                        return Err(self.err_here("FRG-PARSE-002",
                            "An 'if' can have only one 'otherwise'."));
                    }
                    let (body2, stop2) = self.parse_block(BlockEnd::If)?;
                    if matches!(stop2, BlockStop::Otherwise) {
                        return Err(self.err_here("FRG-PARSE-002",
                            "An 'if' can have only one 'otherwise'."));
                    }
                    otherwise = Some(body2);
                    break;
                }
            }
        }
        Ok(Stmt::If { arms, otherwise, line })
    }

    fn parse_create(&mut self, line: usize) -> Result<Stmt, ParseError> {
        self.advance(); // create
        match self.peek().clone() {
            TokenKind::Function => {
                self.advance();
                self.expect(TokenKind::Called)?;
                let (name, _) = self.expect_ident()?;
                let params = self.parse_params()?;
                self.parse_returning()?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::Function))?;
                Ok(Stmt::Function { name, params, body, line })
            }
            TokenKind::Screen => {
                self.advance();
                self.expect(TokenKind::Called)?;
                let name = self.expect_name_value()?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::Screen))?;
                Ok(Stmt::Gui { kind: GuiStmt::CreateScreen { name, body }, line })
            }
            TokenKind::Game => {
                self.advance();
                self.expect(TokenKind::Called)?;
                let name = self.expect_name_value()?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::Game))?;
                Ok(Stmt::Gui { kind: GuiStmt::CreateGame { name, body }, line })
            }
            _ => Err(self.err_here("FRG-PARSE-001",
                "After 'create' FRIDGE expects 'function', 'screen' or 'game'.")),
        }
    }

    fn parse_params(&mut self) -> Result<Vec<String>, ParseError> {
        let mut params = Vec::new();
        if self.is(TokenKind::Taking) {
            self.advance();
            loop {
                let (p, _) = self.expect_ident()?;
                params.push(p);
                if self.is(TokenKind::And) || self.is(TokenKind::Comma) { self.advance(); } else { break; }
            }
        }
        Ok(params)
    }

    fn parse_returning(&mut self) -> Result<(), ParseError> {
        if self.is(TokenKind::Returning) {
            self.advance();
            // Type annotation is parsed and recorded in the spec; the alpha
            // interpreter does not enforce it yet (honest limitation).
            match self.peek().clone() {
                TokenKind::Number | TokenKind::Text | TokenKind::Boolean
                | TokenKind::Anything | TokenKind::Nothing => { self.advance(); }
                TokenKind::List => {
                    self.advance();
                    if self.is(TokenKind::Of) { self.advance(); self.parse_returning_type_inner()?; }
                }
                _ => return Err(self.err_here("FRG-PARSE-001",
                    "Expected a type name such as 'number', 'text' or 'boolean'.")),
            }
        }
        Ok(())
    }

    fn parse_returning_type_inner(&mut self) -> Result<(), ParseError> {
        match self.peek().clone() {
            TokenKind::Number | TokenKind::Text | TokenKind::Boolean
            | TokenKind::Anything | TokenKind::Nothing => { self.advance(); Ok(()) }
            _ => Err(self.err_here("FRG-PARSE-001", "Expected a type name after 'of'.")),
        }
    }

    fn parse_event(&mut self) -> Result<Event, ParseError> {
        if self.is(TokenKind::Program) {
            self.advance();
            if self.is(TokenKind::Starts) { self.advance(); return Ok(Event::ProgramStarts); }
            if self.is(TokenKind::Ends) { self.advance(); return Ok(Event::ProgramEnds); }
            return Err(self.err_here("FRG-PARSE-001",
                "Expected 'starts' or 'ends' after 'program'."));
        }
        if self.is(TokenKind::On) {
            self.advance();
            self.expect(TokenKind::Starts)?;
            return Ok(Event::ProgramStarts);
        }
        if self.is(TokenKind::Each) {
            self.advance();
            self.expect(TokenKind::Frame)?;
            return Ok(Event::EachFrame);
        }
        let target = self.parse_expr(true)?;
        if self.is(TokenKind::Collides) {
            self.advance();
            self.expect(TokenKind::With)?;
            let other = self.parse_expr(true)?;
            return Ok(Event::Collides(target, other));
        }
        self.expect(TokenKind::Is)?;
        if self.is(TokenKind::Pressed) { self.advance(); return Ok(Event::Pressed(target)); }
        if self.is(TokenKind::Released) { self.advance(); return Ok(Event::Released(target)); }
        if self.is(TokenKind::Touched) { self.advance(); return Ok(Event::Touched(target)); }
        Err(self.err_here("FRG-PARSE-006",
            "Unknown event. This build understands: program starts, program ends, each frame, THING is pressed/released/touched, THING collides with THING."))
    }

    // ---------- blocks ----------

    fn parse_block(&mut self, end: BlockEnd) -> Result<(Vec<Stmt>, BlockStop), ParseError> {
        if !self.is(TokenKind::Newline) {
            return Err(self.err_here("FRG-PARSE-003",
                "Put a new line after this instruction header, then the block body."));
        }
        self.skip_newlines();
        let mut stmts = Vec::new();
        loop {
            if self.is(TokenKind::Eof) {
                return Err(self.err_here("FRG-PARSE-002",
                    "This block is never closed. Add the matching 'end ...' line."));
            }
            if self.is(TokenKind::End) {
                self.advance();
                let ok = match &end {
                    BlockEnd::Kw(k) => self.is(k.clone()),
                    BlockEnd::If => self.is(TokenKind::If),
                };
                if ok {
                    self.advance();
                    return Ok((stmts, BlockStop::End));
                }
                let expected = match &end {
                    BlockEnd::Kw(k) => format!("end {}", describe(k)),
                    BlockEnd::If => "end if".to_string(),
                };
                return Err(self.err_here("FRG-PARSE-002",
                    &format!("Expected '{}', found 'end {}'.", expected, describe(self.peek()))));
            }
            if matches!(end, BlockEnd::If) && self.is(TokenKind::Otherwise) {
                return Ok((stmts, BlockStop::Otherwise));
            }
            stmts.push(self.parse_statement()?);
            if !(self.is(TokenKind::Newline) || self.is(TokenKind::Eof)) {
                return Err(self.err_here("FRG-PARSE-003",
                    "Each instruction must be on its own line."));
            }
            self.skip_newlines();
        }
    }

    // ---------- assignable targets ----------

    fn parse_assignable(&mut self) -> Result<Expr, ParseError> {
        let e = self.parse_postfix()?;
        match &e {
            Expr::Var { .. } | Expr::Index { .. } | Expr::Field { .. } => Ok(e),
            _ => Err(self.err_here("FRG-PARSE-005",
                "This cannot be changed. You can change a variable, an item of a list, or a property.")),
        }
    }

    // ---------- expressions ----------
    // Precedence (loosest to tightest):
    // or < and < not < comparison < joined with / to < plus/minus <
    // multiplied by / divided by / modulo < unary minus < of / primary

    fn parse_expr(&mut self, range: bool) -> Result<Expr, ParseError> {
        self.parse_or(range)
    }

    fn parse_or(&mut self, range: bool) -> Result<Expr, ParseError> {
        let mut left = self.parse_and(range)?;
        while self.is(TokenKind::Or) {
            self.advance();
            let right = self.parse_and(range)?;
            left = Expr::Or { left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_and(&mut self, range: bool) -> Result<Expr, ParseError> {
        let mut left = self.parse_not(range)?;
        while self.is(TokenKind::And) {
            self.advance();
            let right = self.parse_not(range)?;
            left = Expr::And { left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_not(&mut self, range: bool) -> Result<Expr, ParseError> {
        if self.is(TokenKind::Not) {
            self.advance();
            Ok(Expr::Not(Box::new(self.parse_not(range)?)))
        } else {
            self.parse_comparison(range)
        }
    }

    fn parse_comparison(&mut self, range: bool) -> Result<Expr, ParseError> {
        let left = self.parse_join(range)?;
        if self.is(TokenKind::Is) {
            let line = self.peek_token().line;
            self.advance();
            let negated = if self.is(TokenKind::Not) { self.advance(); true } else { false };
            let op = if self.is(TokenKind::Greater) {
                self.advance(); self.expect(TokenKind::Than)?; CmpOp::Gt
            } else if self.is(TokenKind::Less) {
                self.advance(); self.expect(TokenKind::Than)?; CmpOp::Lt
            } else if self.is(TokenKind::At) {
                self.advance();
                if self.is(TokenKind::Least) { self.advance(); CmpOp::Ge }
                else if self.is(TokenKind::Most) { self.advance(); CmpOp::Le }
                else {
                    return Err(self.err_here("FRG-PARSE-001",
                        "Expected 'least' or 'most' after 'at'."));
                }
            } else if self.is(TokenKind::Equal) {
                self.advance(); self.expect(TokenKind::To)?; CmpOp::Eq
            } else {
                CmpOp::Eq
            };
            let right = self.parse_join(range)?;
            return Ok(Expr::Cmp { op, negated, left: Box::new(left), right: Box::new(right), line });
        }
        Ok(left)
    }

    fn parse_join(&mut self, range: bool) -> Result<Expr, ParseError> {
        let mut left = self.parse_add();
        loop {
            if self.is(TokenKind::Joined) {
                self.advance();
                self.expect(TokenKind::With)?;
                let right = self.parse_add();
                left = Expr::Concat { left: Box::new(left), right: Box::new(right) };
            } else if range && self.is(TokenKind::To) {
                let line = self.peek_token().line;
                self.advance();
                let right = self.parse_add();
                left = Expr::Range { from: Box::new(left), to: Box::new(right), line };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_mul()?;
        loop {
            let op = if self.is(TokenKind::Plus) {
                BinOp::Add
            } else if self.is(TokenKind::Minus) {
                BinOp::Sub
            } else {
                break;
            };
            let line = self.peek_token().line;
            self.advance();
            let right = self.parse_mul()?;
            left = Expr::Bin { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.is(TokenKind::Multiplied) {
                self.advance(); self.expect(TokenKind::By)?; BinOp::Mul
            } else if self.is(TokenKind::Divided) {
                self.advance(); self.expect(TokenKind::By)?; BinOp::Div
            } else if self.is(TokenKind::Modulo) {
                self.advance(); BinOp::Mod
            } else {
                break;
            };
            let line = self.peek_token().line;
            let right = self.parse_unary()?;
            left = Expr::Bin { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.is(TokenKind::Minus) {
            self.advance();
            Ok(Expr::Neg(Box::new(self.parse_unary()?)))
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let base = self.parse_primary()?;
        if self.is(TokenKind::Of) {
            self.advance();
            let object = self.parse_postfix()?;
            match base {
                Expr::Var { name, line } => Ok(Expr::Field { field: name, object: Box::new(object), line }),
                _ => Err(self.err_here("FRG-PARSE-007",
                    "'of' needs a plain name on the left, like: health of player")),
            }
        } else {
            Ok(base)
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let line = self.peek_token().line;
        match self.peek().clone() {
            TokenKind::Int(v) => { self.advance(); Ok(Expr::Int(v)) }
            TokenKind::Float(v) => { self.advance(); Ok(Expr::Float(v)) }
            TokenKind::Str(s) => { self.advance(); Ok(Expr::Str(s)) }
            TokenKind::True => { self.advance(); Ok(Expr::Bool(true)) }
            TokenKind::False => { self.advance(); Ok(Expr::Bool(false)) }
            TokenKind::Nothing => { self.advance(); Ok(Expr::Nothing) }
            TokenKind::Ident(name) => { self.advance(); Ok(Expr::Var { name, line }) }
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr(true)?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::Call => {
                self.advance();
                let (name, _) = self.expect_ident()?;
                let args = self.parse_args()?;
                Ok(Expr::Call { name, args, line })
            }
            TokenKind::Empty => {
                self.advance();
                self.expect(TokenKind::List)?;
                if self.is(TokenKind::Of) { self.advance(); self.parse_returning_type_inner()?; }
                Ok(Expr::List { items: Vec::new(), line })
            }
            TokenKind::List => {
                self.advance();
                let mut items = Vec::new();
                if self.is(TokenKind::Containing) {
                    self.advance();
                    loop {
                        items.push(self.parse_expr(true)?);
                        if self.is(TokenKind::Comma) { self.advance(); } else { break; }
                    }
                }
                Ok(Expr::List { items, line })
            }
            TokenKind::Item => {
                self.advance();
                let index = self.parse_add()?;
                self.expect(TokenKind::Of)?;
                let list = self.parse_postfix()?;
                Ok(Expr::Index { index: Box::new(index), list: Box::new(list), line })
            }
            TokenKind::First => {
                self.advance();
                self.expect(TokenKind::Item)?;
                self.expect(TokenKind::In)?;
                let list = self.parse_postfix()?;
                Ok(Expr::First { list: Box::new(list), line })
            }
            TokenKind::Last => {
                self.advance();
                self.expect(TokenKind::Item)?;
                self.expect(TokenKind::In)?;
                let list = self.parse_postfix()?;
                Ok(Expr::Last { list: Box::new(list), line })
            }
            TokenKind::Number => {
                self.advance();
                self.expect(TokenKind::Of)?;
                self.expect(TokenKind::Items)?;
                self.expect(TokenKind::In)?;
                let list = self.parse_postfix()?;
                Ok(Expr::Count { list: Box::new(list), line })
            }
            _ => Err(self.err_here("FRG-PARSE-001",
                &format!("Expected a value, found '{}'.", describe(self.peek())))),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if self.is(TokenKind::With) {
            self.advance();
            loop {
                args.push(self.parse_expr(true)?);
                if self.is(TokenKind::Comma) { self.advance(); } else { break; }
            }
        }
        Ok(args)
    }
}

fn describe(k: &TokenKind) -> String {
    use TokenKind::*;
    match k {
        Eof => "end of file".into(),
        Newline => "new line".into(),
        Comma => ",".into(),
        LParen => "(".into(),
        RParen => ")".into(),
        Int(v) => v.to_string(),
        Float(v) => v.to_string(),
        Str(s) => format!("\"{s}\""),
        Ident(s) => s.clone(),
        Remember => "remember".into(), As => "as".into(), Set => "set".into(), To => "to".into(),
        Add => "add".into(), Subtract => "subtract".into(), Multiply => "multiply".into(),
        Divide => "divide".into(), Append => "append".into(), From => "from".into(), By => "by".into(),
        Show => "show".into(), Call => "call".into(), With => "with".into(), Return => "return".into(),
        Break => "break".into(), Continue => "continue".into(), Use => "use".into(),
        If => "if".into(), Otherwise => "otherwise".into(), End => "end".into(),
        Repeat => "repeat".into(), Times => "times".into(), Forever => "forever".into(),
        While => "while".into(), For => "for".into(), Each => "each".into(), In => "in".into(),
        Create => "create".into(), Function => "function".into(), Called => "called".into(),
        Taking => "taking".into(), And => "and".into(), Returning => "returning".into(),
        When => "when".into(), Or => "or".into(), Not => "not".into(), Is => "is".into(),
        Greater => "greater".into(), Less => "less".into(), Than => "than".into(),
        At => "at".into(), Least => "least".into(), Most => "most".into(), Equal => "equal".into(),
        Joined => "joined".into(), Modulo => "modulo".into(), Plus => "plus".into(),
        Minus => "-".into(), Multiplied => "multiplied".into(), Divided => "divided".into(),
        True => "true".into(), False => "false".into(), Nothing => "nothing".into(),
        Number => "number".into(), Text => "text".into(), Boolean => "boolean".into(),
        Anything => "anything".into(), List => "list".into(), Empty => "empty".into(),
        Containing => "containing".into(), Of => "of".into(), Item => "item".into(),
        Items => "items".into(), First => "first".into(), Last => "last".into(),
        Program => "program".into(), Starts => "starts".into(), Ends => "ends".into(),
        Pressed => "pressed".into(), Released => "released".into(), Touched => "touched".into(),
        Collides => "collides".into(), Frame => "frame".into(),
        Structure => "structure".into(), Map => "map".into(), Game => "game".into(),
        Screen => "screen".into(), Place => "place".into(), Open => "open".into(),
        Showing => "showing".into(), On => "on".into(), Define => "define".into(),
        Becomes => "becomes".into(),
    }
}
