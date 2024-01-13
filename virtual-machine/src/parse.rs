use crate::lex::*;

// 文（Statement）を簡潔に述べると、処理する1ステップが1つの文と言えます。
#[derive(Debug)]
pub enum Statement {
    // 式（Expression）を簡潔に述べると、値を生成し、変数に代入できるものを言います。
    Expression(Expression),
    If(If),
    FunctionDeclaration(FunctionDeclaration),
    Return(Return),
    Local(Local),
}

pub type Ast = Vec<Statement>;

#[derive(Debug)]
pub enum Literal {
    Identifier(Token),
    Number(Token),
}

#[derive(Debug)]
pub struct FunctionCall {
    pub name: Token,
    pub arguments: Vec<Expression>,
}

#[derive(Debug)]
pub struct BinaryOperation {
    pub operator: Token,
    pub left: Box<Expression>, // なぜBox<>で囲む？
    pub right: Box<Expression>,
}

#[derive(Debug)]
pub enum Expression {
    FunctionCall(FunctionCall),
    BinaryOperation(BinaryOperation),
    Literal(Literal),
}

#[derive(Debug)]
pub struct FunctionDeclaration {
    pub name: Token,
    pub parameters: Vec<Token>,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub struct If {
    pub test: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub struct Local {
    pub name: Token,
    pub expression: Expression,
}

#[derive(Debug)]
pub struct Return {
    pub expression: Expression,
}

fn expect_keyword(tokens: &[Token], index: usize, value: &str) -> bool {
    if index >= tokens.len() {
        return false;
    }
    let t = tokens[index].clone();
    t.kind == TokenKind::Keyword && t.value == value
}

fn expect_separator(tokens: &[Token], index: usize, value: &str) -> bool {
    if index >= tokens.len() {
        return false;
    }
    let t = tokens[index].clone();
    t.kind == TokenKind::Separator && t.value == value
}

fn expect_identifier(tokens: &[Token], index: usize) -> bool {
    if index >= tokens.len() {
        return false;
    }
    let t = tokens[index].clone();
    t.kind == TokenKind::Identifier
}

fn parse_statement(raw: &[char], tokens: &[Token], index: usize) -> Option<(Statement, usize)> {
    let parsers = [
        parse_if,
        parse_expression_statement,
        parse_return,
        parse_function,
        parse_local,
    ];

    for parser in parsers {
        let res = parser(raw, tokens, index);
        if res.is_some() {
            return res;
        }
    }

    None
}

pub fn parse(raw: &[char], tokens: Vec<Token>) -> Result<Ast, String> {
    let mut ast = vec![];
    let mut index = 0;
    let num_tokens = tokens.len();
    while index < num_tokens {
        let res = parse_statement(raw, &tokens, index);
        if let Some((statement, next_index)) = res {
            index = next_index;
            ast.push(statement);
            continue;
        }

        return Err(tokens[index]
            .location
            .debug(raw, "Invalid token while parsing:"));
    }

    Ok(ast)
}

fn parse_expression_statement(
    raw: &[char],
    tokens: &[Token],
    index: usize,
) -> Option<(Statement, usize)> {
    let mut next_index = index;
    let res = parse_expression(raw, tokens, next_index)?;

    let (expression, next_next_index) = res;
    next_index = next_next_index;
    if !expect_separator(tokens, next_index, ";") {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected semicolon after expression:")
        );
    }

    next_index += 1; // Skip past semicolon

    Some((Statement::Expression(expression), next_index))
}

fn parse_expression(raw: &[char], tokens: &[Token], index: usize) -> Option<(Expression, usize)> {
    if index > tokens.len() {
        return None;
    }

    let t = tokens[index].clone();
    let left = match t.kind {
        TokenKind::Number => Expression::Literal(Literal::Number(t)),
        TokenKind::Identifier => Expression::Literal(Literal::Identifier(t)),
        _ => {
            return None;
        }
    };

    let mut next_index = index + 1;
    if expect_separator(tokens, next_index, "(") {
        next_index += 1; // Skip past open paren

        // Function Call
        let mut arguments: Vec<Expression> = vec![];
        while !expect_separator(tokens, next_index, ")") {
            if !arguments.is_empty() {
                if !expect_separator(tokens, next_index, ",") {
                    println!(
                        "{}",
                        tokens[next_index]
                            .location
                            .debug(raw, "Expected comma between function call arguments:")
                    );
                    return None;
                }
            }

            let res = parse_expression(raw, tokens, next_index);
            if let Some((argument, next_next_index)) = res {
                next_index = next_next_index;
                arguments.push(argument);
            } else {
                println!(
                    "{}",
                    tokens[next_index]
                        .location
                        .debug(raw, "Expected valid expression in function call arguments:")
                );
                return None;
            }
        }

        next_index += 1; // Skip past close paren
        return Some((
            Expression::FunctionCall(FunctionCall {
                name: tokens[index].clone(),
                arguments: arguments,
            }),
            next_index,
        ));
    }

    // Might be a literal operation
    if next_index >= tokens.len() || tokens[next_index].clone().kind != TokenKind::Operator {
        return Some((left, next_index));
    }

    // Otherwise, it's a binary operation
    let operator = tokens[next_index].clone();
    next_index += 1; // Skip past operator

    if next_index > tokens.len() {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected valid right hand side binary operand:")
        );
        return None;
    }

    let right_token = tokens[next_index].clone();
    let right = match right_token.kind {
        TokenKind::Number => Expression::Literal(Literal::Number(right_token)),
        TokenKind::Identifier => Expression::Literal(Literal::Identifier(right_token)),
        _ => {
            println!(
                "{}",
                tokens[next_index]
                    .location
                    .debug(raw, "Expected valid right hand side binary operand:")
            );
            return None;
        }
    };
    next_index += 1; // Skip past right hand side

    Some((
        Expression::BinaryOperation(BinaryOperation {
            left: Box::new(left),
            operator: operator,
            right: Box::new(right),
        }),
        next_index,
    ))
}

fn parse_function(raw: &[char], tokens: &[Token], index: usize) -> Option<(Statement, usize)> {
    if !expect_keyword(tokens, index, "function") {
        return None;
    }

    let mut next_index = index + 1;
    if !expect_identifier(tokens, next_index) {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected valid identifier for function name:")
        );
        return None;
    }
    let name = tokens[next_index].clone();

    next_index += 1; // Skip past name
    if !expect_separator(tokens, next_index, "(") {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected open parenthesis in function declaration:")
        );
        return None;
    }

    next_index += 1; // Skip past open parenthesis
    let mut parameters: Vec<Token> = vec![];
    while !expect_separator(tokens, next_index, ")") {
        if !parameters.is_empty() {
            if !expect_separator(tokens, next_index, ",") {
                println!(
                    "{}",
                    tokens[next_index]
                        .location
                        .debug(raw, "Expected comma or close parenthesis after parameter in function declaration:")
                );
                return None;
            }

            next_index += 1; // Skip past comma
        }

        parameters.push(tokens[next_index].clone());
        next_index += 1; // Skip past parameter
    }

    next_index += 1; // Skip past close parenthesis
    let mut statements: Vec<Statement> = vec![];
    while !expect_keyword(tokens, next_index, "end") {
        let res = parse_statement(raw, tokens, next_index);
        if let Some((statement, next_next_index)) = res {
            next_index = next_next_index;
            statements.push(statement);
        } else {
            println!(
                "{}",
                tokens[next_index]
                    .location
                    .debug(raw, "Expected valid statement in function declaration:")
            );
            return None;
        }
    }

    next_index += 1; // Skip past end
    Some((
        Statement::FunctionDeclaration(FunctionDeclaration {
            name: name,
            parameters: parameters,
            body: statements,
        }),
        next_index,
    ))
}

fn parse_return(raw: &[char], tokens: &[Token], index: usize) -> Option<(Statement, usize)> {
    if !expect_keyword(tokens, index, "return") {
        return None;
    }

    let mut next_index = index + 1; // Skip past return
    let res = parse_expression(raw, tokens, next_index);
    if res.is_none() {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected valid expression in return statement:")
        );
        return None;
    }

    let (expression, next_next_index) = res.unwrap();
    next_index = next_next_index;
    if !expect_separator(tokens, next_index, ";") {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected semicolon in return statement:")
        );
        return None;
    }
    next_index += 1; // Skip past expression
    Some((
        Statement::Return(Return {
            expression: expression,
        }),
        next_index,
    ))
}

fn parse_local(raw: &[char], tokens: &[Token], index: usize) -> Option<(Statement, usize)> {
    if !expect_keyword(tokens, index, "local") {
        return None;
    }

    let mut next_index = index + 1; // Skip past local
    if !expect_identifier(tokens, next_index) {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected valid identifier for local name:")
        );
        return None;
    }
    let name = tokens[next_index].clone();

    next_index += 1; // Skip past name
    if !expect_separator(tokens, next_index, "=") {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected = syntax after local variable name:")
        );
        return None;
    }
    next_index += 1; // Skip past = syntax

    let res = parse_expression(raw, tokens, next_index);
    if res.is_none() {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected valid expression in local declaration:")
        );
        return None;
    }

    let (expression, next_next_index) = res.unwrap();
    next_index = next_next_index;
    if !expect_separator(tokens, next_index, ";") {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected semicolon in local return statement:")
        );
        return None;
    }
    next_index += 1; // Skip past semicolon

    Some((
        Statement::Local(Local {
            name: name,
            expression: expression,
        }),
        next_index,
    ))
}

fn parse_if(raw: &[char], tokens: &[Token], index: usize) -> Option<(Statement, usize)> {
    if !expect_keyword(tokens, index, "if") {
        return None;
    }

    let mut next_index = index + 1; // Skip past if
    let res = parse_expression(raw, tokens, next_index);
    if res.is_none() {
        println!(
            "{}",
            tokens[next_index]
                .location
                .debug(raw, "Expected valid expression for if test:")
        );
        return None;
    }

    let (test, next_next_index) = res.unwrap();
    next_index = next_next_index;

    if !expect_keyword(tokens, next_index, "then") {
        return None;
    }

    next_index += 1; // Skip past then
    let mut statements: Vec<Statement> = vec![];
    while !expect_keyword(tokens, next_index, "end") {
        let res = parse_statement(raw, tokens, next_index);
        if let Some((statement, next_next_index)) = res {
            next_index = next_next_index;
            statements.push(statement);
        } else {
            println!(
                "{}",
                tokens[next_index]
                    .location
                    .debug(raw, "Expected valid statement in if body:")
            );
            return None;
        }
    }

    next_index += 1; // Skip past end

    Some((
        Statement::If(If {
            test: test,
            body: statements,
        }),
        next_index,
    ))
}
