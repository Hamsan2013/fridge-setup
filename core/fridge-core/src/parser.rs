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
            return (
                None,
                vec![p.err_here(
                    "FRG-PARSE-003",
                    "Each instruction must be on its own line.",
                )],
            );
        }
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

enum BlockEnd {
    Kw(TokenKind),
    If,
}

enum BlockStop {
    End,
    Otherwise,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn peek_token(&self) -> &Token {
        static EOF: std::sync::OnceLock<Token> = std::sync::OnceLock::new();
        self.tokens.get(self.pos).unwrap_or_else(|| {
            EOF.get_or_init(|| Token {
                kind: TokenKind::Eof,
                line: 0,
                col: 0,
            })
        })
    }

    fn advance(&mut self) -> TokenKind {
        let k = self.peek().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        k
    }

    fn is(&self, k: TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(&k)
    }

    fn is_at(&self, n: usize, k: TokenKind) -> bool {
        let d = self
            .tokens
            .get(self.pos + n)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof);
        std::mem::discriminant(d) == std::mem::discriminant(&k)
    }

    fn ident_at(&self, n: usize) -> Option<String> {
        match self.tokens.get(self.pos + n).map(|t| &t.kind) {
            Some(TokenKind::Ident(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn skip_newlines(&mut self) {
        while self.is(TokenKind::Newline) {
            self.pos += 1;
        }
    }

    fn err_here(&self, code: &str, message: &str) -> ParseError {
        let t = self.peek_token();
        ParseError {
            line: t.line,
            col: t.col,
            code: code.to_string(),
            message: message.to_string(),
        }
    }

    fn expect(&mut self, k: TokenKind) -> Result<(), ParseError> {
        if self.is(k.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(self.err_here(
                "FRG-PARSE-001",
                &format!(
                    "Expected '{}', found '{}'.",
                    describe(&k),
                    describe(self.peek())
                ),
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<(String, usize), ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                let line = self.peek_token().line;
                self.advance();
                Ok((name, line))
            }
            _ => Err(self.err_here(
                "FRG-PARSE-001",
                &format!("Expected a name, found '{}'.", describe(self.peek())),
            )),
        }
    }

    fn expect_name_value(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(s) => {
                self.advance();
                Ok(s)
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(s)
            }
            TokenKind::Game => {
                self.advance();
                Ok("game".into())
            }
            TokenKind::Screen => {
                self.advance();
                Ok("screen".into())
            }
            _ => Err(self.err_here(
                "FRG-PARSE-001",
                &format!("Expected a name, found '{}'.", describe(self.peek())),
            )),
        }
    }

    // ---- statements ----

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
                self.advance();
                let value = self.parse_expr(true)?;
                Ok(Stmt::Set {
                    target: Expr::Var { name, line },
                    value,
                    line,
                })
            }
            TokenKind::Show => {
                self.advance();
                let value = self.parse_expr(true)?;
                Ok(Stmt::Show { value, line })
            }
            TokenKind::Add => self.parse_add_statement(line),
            TokenKind::Subtract => {
                self.advance();
                let value = self.parse_expr(false)?;
                self.expect(TokenKind::From)?;
                let target = self.parse_assignable()?;
                Ok(Stmt::Update {
                    op: UpdateOp::SubtractFrom,
                    target,
                    value,
                    line,
                })
            }
            TokenKind::Append => {
                self.advance();
                let value = self.parse_expr(false)?;
                self.expect(TokenKind::To)?;
                let target = self.parse_assignable()?;
                Ok(Stmt::Update {
                    op: UpdateOp::AppendTo,
                    target,
                    value,
                    line,
                })
            }
            TokenKind::Multiply => {
                self.advance();
                let target = self.parse_assignable()?;
                self.expect(TokenKind::By)?;
                let value = self.parse_expr(true)?;
                Ok(Stmt::Update {
                    op: UpdateOp::MultiplyBy,
                    target,
                    value,
                    line,
                })
            }
            TokenKind::Divide => {
                self.advance();
                let target = self.parse_assignable()?;
                self.expect(TokenKind::By)?;
                let value = self.parse_expr(true)?;
                Ok(Stmt::Update {
                    op: UpdateOp::DivideBy,
                    target,
                    value,
                    line,
                })
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
            TokenKind::Break => {
                self.advance();
                Ok(Stmt::Break { line })
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Stmt::Continue { line })
            }
            TokenKind::If => self.parse_if(line),
            TokenKind::Repeat => {
                self.advance();
                let count = if self.is(TokenKind::Forever) {
                    self.advance();
                    None
                } else {
                    let e = self.parse_expr(true)?;
                    if self.is(TokenKind::Times) {
                        self.advance();
                    }
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
                Ok(Stmt::Function {
                    name,
                    params,
                    body,
                    line,
                })
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
                        return Err(self.err_here(
                            "FRG-PARSE-001",
                            "'at' needs coordinates, for example: at x 10 y 20",
                        ));
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
                Ok(Stmt::Gui {
                    kind: GuiStmt::Place { name, at, size },
                    line,
                })
            }
            TokenKind::Open => {
                self.advance();
                let name = self.expect_name_value()?;
                Ok(Stmt::Gui {
                    kind: GuiStmt::Open { name },
                    line,
                })
            }
            _ => Err(self.err_here(
                "FRG-PARSE-001",
                &format!(
                    "'{}' does not start any FRIDGE instruction.",
                    describe(self.peek())
                ),
            )),
        }
    }

    fn parse_add_statement(&mut self, line: usize) -> Result<Stmt, ParseError> {
        if self.is_at(1, TokenKind::Ident(String::new())) && self.is_at(2, TokenKind::Called) {
            self.advance();
            let (kind, _) = self.expect_ident()?;
            self.expect(TokenKind::Called)?;
            let name = self.expect_name_value()?;
            let showing = if self.is(TokenKind::Showing) {
                self.advance();
                Some(self.parse_expr(true)?)
            } else {
                None
            };
            let mut options = Vec::new();
            if self.is(TokenKind::With) {
                self.advance();
                loop {
                    let (key, _) = self.expect_ident()?;
                    let val = self.parse_expr(true)?;
                    options.push((key, val));
                    if self.is(TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            return Ok(Stmt::Gui {
                kind: GuiStmt::AddControl {
                    kind,
                    name,
                    showing,
                    options,
                },
                line,
            });
        }
        self.advance();
        let value = self.parse_expr(false)?;
        self.expect(TokenKind::To)?;
        let target = self.parse_assignable()?;
        Ok(Stmt::Update {
            op: UpdateOp::AddTo,
            target,
            value,
            line,
        })
    }

    fn parse_if(&mut self, line: usize) -> Result<Stmt, ParseError> {
        self.advance();
        let mut arms = Vec::new();
        let mut otherwise = None;
        loop {
            let cond = self.parse_expr(true)?;
            let (body, stop) = self.parse_block(BlockEnd::If)?;
            arms.push((cond, body));
            match stop {
                BlockStop::End => break,
                BlockStop::Otherwise => {
                    self.advance();
                    if self.is(TokenKind::If) {
                        self.advance();
                        continue;
                    }
                    if otherwise.is_some() {
                        return Err(self.err_here(
                            "FRG-PARSE-002",
                            "An 'if' can have only one 'otherwise'.",
                        ));
                    }
                    let (body2, stop2) = self.parse_block(BlockEnd::If)?;
                    if matches!(stop2, BlockStop::Otherwise) {
                        return Err(self.err_here(
                            "FRG-PARSE-002",
                            "An 'if' can have only one 'otherwise'.",
                        ));
                    }
                    otherwise = Some(body2);
                    break;
                }
            }
        }
        Ok(Stmt::If {
            arms,
            otherwise,
            line,
        })
    }

    fn parse_create(&mut self, line: usize) -> Result<Stmt, ParseError> {
        self.advance();
        match self.peek().clone() {
            TokenKind::Function => {
                self.advance();
                self.expect(TokenKind::Called)?;
                let (name, _) = self.expect_ident()?;
                let params = self.parse_params()?;
                self.parse_returning()?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::Function))?;
                Ok(Stmt::Function {
                    name,
                    params,
                    body,
                    line,
                })
            }
            TokenKind::Screen => {
                self.advance();
                self.expect(TokenKind::Called)?;
                let name = self.expect_name_value()?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::Screen))?;
                Ok(Stmt::Gui {
                    kind: GuiStmt::CreateScreen { name, body },
                    line,
                })
            }
            TokenKind::Game => {
                self.advance();
                self.expect(TokenKind::Called)?;
                let name = self.expect_name_value()?;
                let (body, _) = self.parse_block(BlockEnd::Kw(TokenKind::Game))?;
                Ok(Stmt::Gui {
                    kind: GuiStmt::CreateGame { name, body },
                    line,
                })
            }
            _ => Err(self.err_here(
                "FRG-PARSE-001",
                "After 'create' FRIDGE expects 'function', 'screen' or 'game'.",
            )),
        }
    }

    fn parse_params(&mut self) -> Result<Vec<String>, ParseError> {
        let mut params = Vec::new();
        if self.is(TokenKind::Taking) {
            self.advance();
            loop {
                let (p, _) = self.expect_ident()?;
                params.push(p);
                if self.is(TokenKind::And) || self.is(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        Ok(params)
    }

    fn parse_returning(&mut self) -> Result<(), ParseError> {
        if self.is(TokenKind::Returning) {
            self.advance();
            match self.peek().clone() {
                TokenKind::Number
                | TokenKind::Text
                | TokenKind::Boolean
                | TokenKind::Anything
                | TokenKind::Nothing => {
                    self.advance();
                }
                TokenKind::List => {
                    self.advance();
                    if self.is(TokenKind::Of) {
                        self.advance();
                        self.parse_returning_type_inner()?;
                    }
                }
                _ => {
                    return Err(self.err_here(
                        "FRG-PARSE-001",
                        "Expected a type name such as 'number', 'text' or 'boolean'.",
                    ))
                }
            }
        }
        Ok(())
    }

    fn parse_returning_type_inner(&mut self) -> Result<(), ParseError> {
        match self.peek().clone() {
            TokenKind::Number
            | TokenKind::Text
            | TokenKind::Boolean
            | TokenKind::Anything
            | TokenKind::Nothing => {
                self.advance();
                Ok(())
            }
            _ => Err(self.err_here(
                "FRG-PARSE-001",
                "Expected a type name after 'of'.",
            )),
        }
    }

    fn parse_event(&mut self) -> Result<Event, ParseError> {
        if self.is(TokenKind::Program) {
            self.advance();
            if self.is(TokenKind::Starts) {
                self.advance();
                return Ok(Event::ProgramStarts);
            }
            if self.is(TokenKind::Ends) {
                self.advance();
                return Ok(Event::ProgramEnds);
            }
            return Err(self.err_here(
                "FRG-PARSE-001",
                "Expected 'starts' or 'ends' after 'program'.",
            ));
        }
        if self.is(Toke
