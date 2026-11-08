use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub line: usize,
    pub col: usize,
    pub code: String,
    pub message: String,
}

pub fn lex(source: &str) -> (Vec<Token>, Vec<LexError>) {
    let mut lx = Lexer {
        chars: source.chars().collect(),
        pos: 0,
        line: 1,
        col: 1,
    };
    let mut tokens: Vec<Token> = Vec::new();
    let mut errors: Vec<LexError> = Vec::new();

    loop {
        match lx.peek() {
            None => break,
            Some(' ') | Some('\t') | Some('\r') => { lx.advance(); }
            Some('\n') => lx.consume_newlines(&mut tokens, &mut errors),
            Some('/') => lx.handle_slash(&mut errors),
            Some('"') => lx.read_text(&mut tokens, &mut errors),
            Some(c) if c.is_ascii_digit() => lx.read_number(&mut tokens, &mut errors),
            Some(',') => { let t = lx.here(); lx.advance(); tokens.push(Token { kind: TokenKind::Comma, line: t.0, col: t.1 }); }
            Some('(') => { let t = lx.here(); lx.advance(); tokens.push(Token { kind: TokenKind::LParen, line: t.0, col: t.1 }); }
            Some(')') => { let t = lx.here(); lx.advance(); tokens.push(Token { kind: TokenKind::RParen, line: t.0, col: t.1 }); }
            Some('-') => { let t = lx.here(); lx.advance(); tokens.push(Token { kind: TokenKind::Minus, line: t.0, col: t.1 }); }
            Some(c) if c.is_ascii_alphabetic() || c == '_' => lx.read_word(&mut tokens),
            Some(other) => {
                let (l, c) = lx.here();
                errors.push(LexError {
                    line: l, col: c,
                    code: "FRG-LEX-001".into(),
                    message: format!("Unrecognized character '{other}'. FRIDGE uses letters, digits, commas, parentheses, hyphen and quotation marks."),
                });
                lx.advance();
            }
        }
    }

    while matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Newline)) {
        tokens.pop();
    }
    let (l, c) = lx.here();
    tokens.push(Token { kind: TokenKind::Eof, line: l, col: c });
    (tokens, errors)
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn peek_at(&self, n: usize) -> Option<char> { self.chars.get(self.pos + n).copied() }
    fn here(&self) -> (usize, usize) { (self.line, self.col) }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' { self.line += 1; self.col = 1; } else { self.col += 1; }
        Some(ch)
    }

    fn consume_newlines(&mut self, tokens: &mut Vec<Token>, errors: &mut Vec<LexError>) {
        let (line, col) = self.here();
        loop {
            match self.peek() {
                Some('\n') | Some(' ') | Some('\t') | Some('\r') => { self.advance(); }
                Some('/') if self.peek_at(1) == Some('/') => self.skip_line_comment(),
                Some('/') if self.peek_at(1) == Some('*') => self.skip_block_comment(errors),
                _ => break,
            }
        }
        if self.peek().is_none() { return; }
        if matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Newline) | None) { return; }
        tokens.push(Token { kind: TokenKind::Newline, line, col });
    }

    fn handle_slash(&mut self, errors: &mut Vec<LexError>) {
        match self.peek_at(1) {
            Some('/') => self.skip_line_comment(),
            Some('*') => self.skip_block_comment(errors),
            _ => {
                let (l, c) = self.here();
                errors.push(LexError {
                    line: l, col: c,
                    code: "FRG-LEX-001".into(),
                    message: "Unrecognized character '/'. Did you mean a comment starting with '//' ?".into(),
                });
                self.advance();
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' { break; }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self, errors: &mut Vec<LexError>) {
        let (line, col) = self.here();
        self.advance();
        self.advance();
        loop {
            match self.peek() {
                None => {
                    errors.push(LexError {
                        line, col,
                        code: "FRG-LEX-003".into(),
                        message: "Block comment is never closed. Add '*/' to end it.".into(),
                    });
                    return;
                }
                Some('*') if self.peek_at(1) == Some('/') => { self.advance(); self.advance(); return; }
                Some(_) => { self.advance(); }
            }
        }
    }

    fn read_text(&mut self, tokens: &mut Vec<Token>, errors: &mut Vec<LexError>) {
        let (line, col) = self.here();
        self.advance();
        let mut s = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    errors.push(LexError {
                        line, col,
                        code: "FRG-LEX-002".into(),
                        message: "Text value is never closed. Add a matching '\"'.".into(),
                    });
                    break;
                }
                Some('"') => { self.advance(); break; }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(other) => {
                            errors.push(LexError {
                                line: self.line, col: self.col.saturating_sub(1),
                                code: "FRG-LEX-004".into(),
                                message: format!("Unknown escape '\\{other}'. Supported: \\n \\t \\\" \\\\"),
                            });
                            s.push(other);
                        }
                        None => {
                            errors.push(LexError {
                                line, col,
                                code: "FRG-LEX-002".into(),
                                message: "Text value is never closed. Add a matching '\"'.".into(),
                            });
                            break;
                        }
                    }
                }
                Some(c) => { self.advance(); s.push(c); }
            }
        }
        tokens.push(Token { kind: TokenKind::Str(s), line, col });
    }

    fn read_number(&mut self, tokens: &mut Vec<Token>, errors: &mut Vec<LexError>) {
        let (line, col) = self.here();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() { text.push(c); self.advance(); } else { break; }
        }
        if self.peek() == Some('.') && self.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
            text.push('.');
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() { text.push(c); self.advance(); } else { break; }
            }
            match text.parse::<f64>() {
                Ok(v) => tokens.push(Token { kind: TokenKind::Float(v), line, col }),
                Err(_) => errors.push(LexError { line, col, code: "FRG-LEX-005".into(), message: format!("Cannot read number '{text}'.") }),
              match text.parse::<f64>() {
                Ok(v) => tokens.push(Token { kind: TokenKind::Float(v), line, col }),
                Err(_) => errors.push(LexError {
                    line, col,
                    code: "FRG-LEX-005".into(),
                    message: format!("Cannot read number '{text}'."),
                }),
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => tokens.push(Token { kind: TokenKind::Int(v), line, col }),
                Err(_) => errors.push(LexError {
                    line, col,
                    code: "FRG-LEX-005".into(),
                    message: format!("Number '{text}' is too large for FRIDGE (max 9223372036854775807)."),
                }),
            }
        }
    }

    fn read_word(&mut self, tokens: &mut Vec<Token>) {
        let (line, col) = self.here();
        let mut word = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                word.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match word.as_str() {
            "a" | "an" | "the" => return,
            "note" => { self.skip_line_comment(); return; }
            _ => {}
        }
        let kind = keyword(&word).unwrap_or_else(|| TokenKind::Ident(word));
        tokens.push(Token { kind, line, col });
    }
}

fn keyword(word: &str) -> Option<TokenKind> {
    use TokenKind::*;
    Some(match word {
        "remember" => Remember, "as" => As, "set" => Set, "to" => To,
        "add" => Add, "subtract" => Subtract, "multiply" => Multiply, "divide" => Divide,
        "append" => Append, "from" => From, "by" => By,
        "show" => Show, "call" => Call, "with" => With, "return" => Return,
        "break" => Break, "continue" => Continue, "use" => Use,
        "if" => If, "otherwise" => Otherwise, "end" => End,
        "repeat" => Repeat, "times" => Times, "forever" => Forever,
        "while" => While, "for" => For, "each" => Each, "in" => In,
        "create" => Create, "function" => Function, "called" => Called,
        "taking" => Taking, "and" => And, "returning" => Returning, "when" => When,
        "or" => Or, "not" => Not, "is" => Is, "greater" => Greater, "less" => Less,
        "than" => Than, "at" => At, "least" => Least, "most" => Most, "equal" => Equal,
        "joined" => Joined, "modulo" => Modulo, "plus" => Plus, "minus" => Minus,
        "multiplied" => Multiplied, "divided" => Divided,
        "true" => True, "false" => False, "nothing" => Nothing,
        "number" => Number, "text" => Text, "boolean" => Boolean, "anything" => Anything,
        "list" => List, "empty" => Empty, "containing" => Containing,
        "of" => Of, "item" => Item, "items" => Items, "first" => First, "last" => Last,
        "program" => Program, "starts" => Starts, "ends" => Ends,
        "pressed" => Pressed, "released" => Released, "touched" => Touched,
        "collides" => Collides, "frame" => Frame,
        "structure" => Structure, "map" => Map, "game" => Game, "screen" => Screen,
        "place" => Place, "open" => Open, "showing" => Showing,
        "on" => On, "define" => Define, "becomes" => Becomes,
        // Official shortcuts (spec v0.2.1 §1)
        "else" => Otherwise, "loop" => Repeat, "print" => Show, "start" => Starts,
        _ => return None,
    })
}
