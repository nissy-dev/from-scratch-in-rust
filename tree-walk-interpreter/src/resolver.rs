use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::{
        AssignExpr, BinaryExpr, BlockStmt, CallExpr, Expr, ExprStmt, FunctionDeclStmt,
        GroupingExpr, IfStmt, LogicalExpr, PrintStmt, ReturnStmt, Stmt, UnaryExpr, VarDeclStmt,
        VariableExpr, WhileStmt,
    },
    interpreter::Interpreter,
    lexer::Token,
};

pub struct Resolver {
    interpreter: Rc<RefCell<Interpreter>>,
    scopes: Vec<HashMap<String, bool>>,
}

impl Resolver {
    pub fn new(interpreter: Rc<RefCell<Interpreter>>) -> Self {
        Resolver {
            interpreter,
            scopes: Vec::new(),
        }
    }

    pub fn resolve(&mut self, stmts: Vec<Stmt>) {
        for stmt in stmts {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_stmt(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::Block(block) => self.visit_block_stmt(*block),
            Stmt::VarDecl(var_decl) => self.visit_var_decl_stmt(*var_decl),
            Stmt::FunctionDecl(func_decl) => self.visit_function_decl_stmt(*func_decl),
            Stmt::Expr(expr) => self.visit_expr_stmt(*expr),
            Stmt::If(if_stmt) => self.visit_if_stmt(*if_stmt),
            Stmt::Print(print) => self.visit_print_stmt(*print),
            Stmt::Return(return_stmt) => self.visit_return_stmt(*return_stmt),
            Stmt::While(while_stmt) => self.visit_while_stmt(*while_stmt),
        }
    }

    fn visit_block_stmt(&mut self, block: BlockStmt) {
        self.begin_scope();
        self.resolve(block.statements);
        self.end_scope();
    }

    fn visit_var_decl_stmt(&mut self, var_decl: VarDeclStmt) {
        self.declare(var_decl.name.clone());
        if let Some(initializer) = var_decl.initializer {
            self.resolve_expr(initializer);
        }
        self.define(var_decl.name);
    }

    fn visit_function_decl_stmt(&mut self, func_decl: FunctionDeclStmt) {
        self.declare(func_decl.name.clone());
        self.define(func_decl.name.clone());
        self.resolve_function(func_decl);
    }

    fn visit_expr_stmt(&mut self, expr: ExprStmt) {
        self.resolve_expr(expr.expr);
    }

    fn visit_if_stmt(&mut self, if_stmt: IfStmt) {
        self.resolve_expr(if_stmt.condition);
        self.resolve_stmt(*if_stmt.then_branch);
        if let Some(else_branch) = if_stmt.else_branch {
            self.resolve_stmt(*else_branch);
        }
    }

    fn visit_print_stmt(&mut self, print: PrintStmt) {
        self.resolve_expr(print.expr);
    }

    fn visit_return_stmt(&mut self, return_stmt: ReturnStmt) {
        if let Some(value) = return_stmt.value {
            self.resolve_expr(value);
        }
    }

    fn visit_while_stmt(&mut self, while_stmt: WhileStmt) {
        self.resolve_expr(while_stmt.condition);
        self.resolve_stmt(*while_stmt.body);
    }

    fn resolve_function(&mut self, func_decl: FunctionDeclStmt) {
        self.begin_scope();
        for param in &func_decl.params {
            self.declare(param.clone());
            self.define(param.clone());
        }
        self.resolve(func_decl.body);
        self.end_scope();
    }

    fn resolve_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Variable(variable) => self.visit_variable_expression(*variable),
            Expr::Assign(assign) => self.visit_assign_expression(*assign),
            Expr::Binary(binary) => self.visit_binary_expression(*binary),
            Expr::Call(call) => self.visit_call_expression(*call),
            Expr::Grouping(grouping) => self.visit_grouping_expression(*grouping),
            Expr::Logical(logical) => self.visit_logical_expression(*logical),
            Expr::Unary(unary) => self.visit_unary_expression(*unary),
            Expr::Literal(_) => {} // do nothing
        }
    }

    fn visit_variable_expression(&mut self, variable: VariableExpr) {
        if let Some(scope) = self.scopes.last() {
            if let Some(declared) = scope.get(&variable.name.lexeme) {
                if !declared {
                    tracing::error!(
                        "Cannot read local variable in its own initializer: {}",
                        variable.name.lexeme
                    );
                }
            }
        }

        let expr = Expr::Variable(Box::new(variable.clone()));
        self.resolve_local(expr, variable.name);
    }

    fn visit_assign_expression(&mut self, assign: AssignExpr) {
        self.resolve_expr(assign.value.clone());
        let expr = Expr::Assign(Box::new(assign.clone()));
        self.resolve_local(expr, assign.name);
    }

    fn visit_binary_expression(&mut self, binary: BinaryExpr) {
        self.resolve_expr(binary.left);
        self.resolve_expr(binary.right);
    }

    fn visit_call_expression(&mut self, call: CallExpr) {
        self.resolve_expr(call.callee);
        for arg in call.arguments {
            self.resolve_expr(arg);
        }
    }

    fn visit_grouping_expression(&mut self, grouping: GroupingExpr) {
        self.resolve_expr(grouping.expression);
    }

    fn visit_logical_expression(&mut self, logical: LogicalExpr) {
        self.resolve_expr(logical.left);
        self.resolve_expr(logical.right);
    }

    fn visit_unary_expression(&mut self, unary: UnaryExpr) {
        self.resolve_expr(unary.right);
    }

    fn resolve_local(&mut self, expr: Expr, name: Token) {
        for (i, scope) in self.scopes.iter().rev().enumerate() {
            if scope.contains_key(&name.lexeme) {
                self.interpreter.borrow_mut().resolve(expr, i);
                return;
            }
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, token: Token) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(token.lexeme, false);
        }
    }

    fn define(&mut self, token: Token) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(token.lexeme, true);
        }
    }
}
