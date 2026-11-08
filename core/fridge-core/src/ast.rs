#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Remember { name: String, value: Expr, line: usize },
    Set { target: Expr, value: Expr, line: usize },
    Update { op: UpdateOp, target: Expr, value: Expr, line: usize },
    Show { value: Expr, line: usize },
    Call { name: String, args: Vec<Expr>, line: usize },
    Return { value: Option<Expr>, line: usize },
    Break { line: usize },
    Continue { line: usize },
    If { arms: Vec<(Expr, Vec<Stmt>)>, otherwise: Option<Vec<Stmt>>, line: usize },
    Repeat { count: Option<Expr>, body: Vec<Stmt>, line: usize },
    While { cond: Expr, body: Vec<Stmt>, line: usize },
    ForEach { var: String, iter: Expr, body: Vec<Stmt>, line: usize },
    Function { name: String, params: Vec<String>, body: Vec<Stmt>, line: usize },
    When { event: Event, body: Vec<Stmt>, line: usize },
    Use { module: String, line: usize },
    Gui { kind: GuiStmt, line: usize },
}

#[derive(Debug, Clone, Copy)]
pub enum UpdateOp {
    AddTo,
    SubtractFrom,
    MultiplyBy,
    DivideBy,
    AppendTo,
}

#[derive(Debug, Clone)]
pub enum Event {
    ProgramStarts,
    ProgramEnds,
    EachFrame,
    Pressed(Expr),
    Released(Expr),
    Touched(Expr),
    Collides(Expr, Expr),
}

#[derive(Debug, Clone)]
pub enum GuiStmt {
    CreateScreen { name: String, body: Vec<Stmt> },
    CreateGame { name: String, body: Vec<Stmt> },
    AddControl {
        kind: String,
        name: String,
        showing: Option<Expr>,
        options: Vec<(String, Expr)>,
    },
    Place {
        name: String,
        at: Vec<(String, Expr)>,
        size: Vec<(String, Expr)>,
    },
    Open { name: String },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nothing,
    Var { name: String, line: usize },
    List { items: Vec<Expr>, line: usize },
    Range { from: Box<Expr>, to: Box<Expr>, line: usize },
    Concat { left: Box<Expr>, right: Box<Expr> },
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin { op: BinOp, left: Box<Expr>, right: Box<Expr>, line: usize },
    Cmp { op: CmpOp, negated: bool, left: Box<Expr>, right: Box<Expr>, line: usize },
    And { left: Box<Expr>, right: Box<Expr> },
    Or { left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Expr>, line: usize },
    Index { index: Box<Expr>, list: Box<Expr>, line: usize },
    First { list: Box<Expr>, line: usize },
    Last { list: Box<Expr>, line: usize },
    Count { list: Box<Expr>, line: usize },
    Field { field: String, object: Box<Expr>, line: usize },
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp { Add, Sub, Mul, Div, Mod }

#[derive(Debug, Clone, Copy)]
pub enum CmpOp { Eq, Gt, Lt, Ge, Le }
