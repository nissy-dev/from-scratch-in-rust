use crate::lexer::Token;

#[derive(Debug)]
pub enum Expr {
    Literal(Box<LiteralExpr>),
    Unary(Box<UnaryExpr>),
    Binary(Box<BinaryExpr>),
    Grouping(Box<GroupingExpr>),
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
