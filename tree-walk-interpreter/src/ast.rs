use crate::lexer::Token;

#[derive(Debug)]
pub enum Stmt {
    Expr(Box<ExprStmt>),
    Print(Box<PrintStmt>),
    VarDecl(Box<VarDeclStmt>),
    Block(Box<BlockStmt>),
}

#[derive(Debug)]
pub struct ExprStmt {
    pub expr: Expr,
}

impl ExprStmt {
    pub fn new(expr: Expr) -> Self {
        ExprStmt { expr }
    }
}

#[derive(Debug)]
pub struct PrintStmt {
    pub expr: Expr,
}

impl PrintStmt {
    pub fn new(expr: Expr) -> Self {
        PrintStmt { expr }
    }
}

#[derive(Debug)]
pub struct VarDeclStmt {
    pub name: Token,
    pub initializer: Option<Expr>,
}

impl VarDeclStmt {
    pub fn new(name: Token, initializer: Option<Expr>) -> Self {
        VarDeclStmt { name, initializer }
    }
}

#[derive(Debug)]
pub struct BlockStmt {
    pub statements: Vec<Stmt>,
}

impl BlockStmt {
    pub fn new(statements: Vec<Stmt>) -> Self {
        BlockStmt { statements }
    }
}

#[derive(Debug)]
pub enum Expr {
    Literal(Box<LiteralExpr>),
    Unary(Box<UnaryExpr>),
    Binary(Box<BinaryExpr>),
    Grouping(Box<GroupingExpr>),
    Variable(Box<VariableExpr>),
    Assign(Box<AssignExpr>),
}

#[derive(Debug)]
pub struct LiteralExpr {
    pub value: Token,
}

impl LiteralExpr {
    pub fn new(value: Token) -> Self {
        LiteralExpr { value }
    }
}

#[derive(Debug)]
pub struct BinaryExpr {
    pub left: Expr,
    pub operator: Token,
    pub right: Expr,
}

impl BinaryExpr {
    pub fn new(left: Expr, operator: Token, right: Expr) -> Self {
        BinaryExpr {
            left,
            operator,
            right,
        }
    }
}

#[derive(Debug)]
pub struct UnaryExpr {
    pub operator: Token,
    pub right: Expr,
}

impl UnaryExpr {
    pub fn new(operator: Token, right: Expr) -> Self {
        UnaryExpr { operator, right }
    }
}

#[derive(Debug)]
pub struct GroupingExpr {
    pub expression: Expr,
}

impl GroupingExpr {
    pub fn new(expression: Expr) -> Self {
        GroupingExpr { expression }
    }
}

#[derive(Debug)]
pub struct VariableExpr {
    pub name: Token,
}

impl VariableExpr {
    pub fn new(name: Token) -> Self {
        VariableExpr { name }
    }
}

#[derive(Debug)]
pub struct AssignExpr {
    pub name: Token,
    pub value: Expr,
}

impl AssignExpr {
    pub fn new(name: Token, value: Expr) -> Self {
        AssignExpr { name, value }
    }
}
