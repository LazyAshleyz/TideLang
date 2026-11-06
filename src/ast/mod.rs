#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Bn,
    Ft,
    Str,
    Array,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg, // -x
    Not, // not x
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Assign,    // =
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
    ModAssign, // %=
}

// Things that produce a value: 5, 3.14, "hi", true, x, -x, a + b, [1, 2], nums[0], std.len(x)
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Variable(String),
    Array(Vec<Expr>),
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        module: Option<String>, // Some("std") for std.len(...), None for plain calls
        name: String,
        args: Vec<Expr>,
    },
}

// Things that do something: declare, assign, branch, loop, return, import
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Import(String),
    VarDecl {
        ty: Type,
        name: String,
        value: Expr,
    },
    Assign {
        name: String,
        op: AssignOp,
        value: Expr,
    },
    IndexAssign {
        name: String,
        index: Expr,
        op: AssignOp,
        value: Expr,
    },
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    // for (init; condition; step) { body }   (every part of the header is optional)
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        step: Option<Box<Stmt>>,
        body: Vec<Stmt>,
    },
    // for (int x in collection) { body }
    ForIn {
        ty: Type,
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Return(Option<Expr>),
    Function(FunctionDecl),
    ExprStmt(Expr), // e.g. a bare call like `std.push(nums, 4);`
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}

// A whole .td file is just a list of statements
pub type Program = Vec<Stmt>;