#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Eof,
    Newline,
    Comma,
    LParen,
    RParen,

    Int(i64),
    Float(f64),
    Str(String),
    Ident(String),

    Remember, As, Set, To, Add, Subtract, Multiply, Divide, Append, From, By,
    Show, Call, With, Return, Break, Continue, Use,

    If, Otherwise, End, Repeat, Times, Forever, While, For, Each, In,

    Create, Function, Called, Taking, And, Returning, When,

    Or, Not, Is, Greater, Less, Than, At, Least, Most, Equal, Joined, Modulo,
    Plus, Minus, Multiplied, Divided,

    True, False, Nothing,

    Number, Text, Boolean, Anything, List, Empty, Containing, Of, Item, Items,
    First, Last,

    Program, Starts, Ends, Pressed, Released, Touched, Collides, Frame,

    Structure, Map, Game, Screen, Place, Open, Showing,

    On, Define, Becomes,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}
