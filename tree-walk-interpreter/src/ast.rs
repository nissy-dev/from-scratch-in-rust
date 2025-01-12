use crate::lexer::Token;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Stmt {
    Expr(Box<ExprStmt>),
    Print(Box<PrintStmt>),
    VarDecl(Box<VarDeclStmt>),
    FunctionDecl(Box<FunctionDeclStmt>),
    ClassDecl(Box<ClassDeclStmt>),
    Block(Box<BlockStmt>),
    If(Box<IfStmt>),
    While(Box<WhileStmt>),
    Return(Box<ReturnStmt>),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ExprStmt {
    pub expr: Expr,
}

impl ExprStmt {
    pub fn new(expr: Expr) -> Self {
        ExprStmt { expr }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PrintStmt {
    pub expr: Expr,
}

impl PrintStmt {
    pub fn new(expr: Expr) -> Self {
        PrintStmt { expr }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct VarDeclStmt {
    pub name: Token,
    pub initializer: Option<Expr>,
}

impl VarDeclStmt {
    pub fn new(name: Token, initializer: Option<Expr>) -> Self {
        VarDeclStmt { name, initializer }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FunctionDeclStmt {
    pub name: Token,
    pub params: Vec<Token>,
    pub body: Vec<Stmt>,
}

impl FunctionDeclStmt {
    pub fn new(name: Token, params: Vec<Token>, body: Vec<Stmt>) -> Self {
        FunctionDeclStmt { name, params, body }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ClassDeclStmt {
    pub name: Token,
    pub methods: Vec<Stmt>,
}

impl ClassDeclStmt {
    pub fn new(name: Token, methods: Vec<Stmt>) -> Self {
        ClassDeclStmt { name, methods }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct BlockStmt {
    pub statements: Vec<Stmt>,
}

impl BlockStmt {
    pub fn new(statements: Vec<Stmt>) -> Self {
        BlockStmt { statements }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Box<Stmt>,
    pub else_branch: Option<Box<Stmt>>,
}

impl IfStmt {
    pub fn new(condition: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>>) -> Self {
        IfStmt {
            condition,
            then_branch,
            else_branch,
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Box<Stmt>,
}

impl WhileStmt {
    pub fn new(condition: Expr, body: Box<Stmt>) -> Self {
        WhileStmt { condition, body }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ReturnStmt {
    pub keyword: Token,
    pub value: Option<Expr>,
}

impl ReturnStmt {
    pub fn new(keyword: Token, value: Option<Expr>) -> Self {
        ReturnStmt { keyword, value }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Expr {
    Literal(Box<LiteralExpr>),
    Unary(Box<UnaryExpr>),
    Binary(Box<BinaryExpr>),
    Grouping(Box<GroupingExpr>),
    Variable(Box<VariableExpr>),
    Assign(Box<AssignExpr>),
    Logical(Box<LogicalExpr>),
    Call(Box<CallExpr>),
    Get(Box<GetExpr>),
    Set(Box<SetExpr>),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct LiteralExpr {
    pub value: Token,
}

impl LiteralExpr {
    pub fn new(value: Token) -> Self {
        LiteralExpr { value }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
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

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct UnaryExpr {
    pub operator: Token,
    pub right: Expr,
}

impl UnaryExpr {
    pub fn new(operator: Token, right: Expr) -> Self {
        UnaryExpr { operator, right }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct GroupingExpr {
    pub expression: Expr,
}

impl GroupingExpr {
    pub fn new(expression: Expr) -> Self {
        GroupingExpr { expression }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct VariableExpr {
    pub name: Token,
}

impl VariableExpr {
    pub fn new(name: Token) -> Self {
        VariableExpr { name }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct AssignExpr {
    pub name: Token,
    pub value: Expr,
}

impl AssignExpr {
    pub fn new(name: Token, value: Expr) -> Self {
        AssignExpr { name, value }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct LogicalExpr {
    pub left: Expr,
    pub operator: Token,
    pub right: Expr,
}

impl LogicalExpr {
    pub fn new(left: Expr, operator: Token, right: Expr) -> Self {
        LogicalExpr {
            left,
            operator,
            right,
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CallExpr {
    pub callee: Expr,
    pub paren: Token,
    pub arguments: Vec<Expr>,
}

impl CallExpr {
    pub fn new(callee: Expr, paren: Token, arguments: Vec<Expr>) -> Self {
        CallExpr {
            callee,
            paren,
            arguments,
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct GetExpr {
    pub object: Expr,
    pub name: Token,
}

impl GetExpr {
    pub fn new(object: Expr, name: Token) -> Self {
        GetExpr { object, name }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct SetExpr {
    pub object: Expr,
    pub name: Token,
    pub value: Expr,
}

impl SetExpr {
    pub fn new(object: Expr, name: Token, value: Expr) -> Self {
        SetExpr {
            object,
            name,
            value,
        }
    }
}
