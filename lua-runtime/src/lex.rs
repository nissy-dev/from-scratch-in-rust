#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    column: i32,
    line: i32,
    index: usize,
}

impl Location {
    fn increment(&self, newline: bool) -> Location {
        if newline {
            Location {
                column: 0,
                line: self.line + 1,
                index: self.index + 1,
            }
        } else {
            Location {
                column: self.column + 1,
                line: self.line,
                index: self.index + 1,
            }
        }
    }

    // dump the current line with a pointer in text to the current column along with a message.
    pub fn debug<S: Into<String>>(&self, raw: &[char], msg: S) -> String {
        let mut line = 0;
        let mut line_str = String::new();
        // Find the whole line of original source
        for c in raw {
            if *c == '\n' {
                line += 1;

                // Done discovering line in question
                if !line_str.is_empty() {
                    break;
                }

                continue;
            }
            if self.line == line {
                line_str.push_str(&c.to_string());
            }
        }
        let space = " ".repeat(self.column as usize);
        format!("{}\n\n{}\n{}^ Near here", msg.into(), line_str, space)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Identifier,
    Separator,
    Keyword,
    Number,
    Operator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub value: String,
    pub kind: TokenKind,
    pub location: Location,
}

pub fn lex(s: &[char]) -> Result<Vec<Token>, String> {
    let mut loc = Location {
        column: 0,
        line: 0,
        index: 0,
    };
    let size = s.len();
    let mut tokens = vec![];

    let lexers = [
        lex_keyword,
        lex_identifier,
        lex_number,
        lex_separator,
        lex_operator,
    ];

    'outer: while loc.index < size {
        loc = eat_whitespaces(s, loc);
        if loc.index == size {
            break;
        }

        for lexer in lexers {
            let res = lexer(s, loc);
            if let Some((t, next_c)) = res {
                loc = next_c;
                tokens.push(t);
                continue 'outer;
            }
        }

        return Err(loc.debug(s, "Unrecognized character while lexing:"));
    }

    Ok(tokens)
}

// スペース or 改行があった時に新しいLocationを返す
fn eat_whitespaces(raw: &[char], loc: Location) -> Location {
    let mut c = raw[loc.index];
    let mut next_loc = loc;
    while [' ', '\t', '\n', '\r'].contains(&c) {
        next_loc = next_loc.increment(c == '\n');
        if next_loc.index == raw.len() {
            break;
        }
        c = raw[next_loc.index];
    }

    next_loc
}

fn lex_identifier(raw: &[char], loc: Location) -> Option<(Token, Location)> {
    let mut c = raw[loc.index];
    let mut next_loc = loc;
    let mut value = String::new();

    while c.is_alphanumeric() || c == '_' {
        value.push(c);
        next_loc = next_loc.increment(false);
        c = raw[next_loc.index];
    }

    // First character must not be a digit
    if value.len() > 0 && !value.chars().next().unwrap().is_digit(10) {
        Some((
            Token {
                value,
                kind: TokenKind::Identifier,
                location: loc,
            },
            next_loc,
        ))
    } else {
        None
    }
}

fn lex_separator(raw: &[char], loc: Location) -> Option<(Token, Location)> {
    let syntax = [";", "=", "(", ")", ","];
    for possible_syntax in syntax {
        let c = raw[loc.index];
        let next_loc = loc.increment(false);
        // TODO: this won't work with multiple-character syntax bits like >= or ==
        if c.to_string() == possible_syntax {
            return Some((
                Token {
                    value: possible_syntax.to_string(),
                    kind: TokenKind::Separator,
                    location: loc,
                },
                next_loc,
            ));
        }
    }

    None
}

fn lex_keyword(raw: &[char], loc: Location) -> Option<(Token, Location)> {
    let keyword = ["function", "end", "if", "then", "local", "return"].map(|val| val.to_string());
    let mut c = raw[loc.index];
    let mut next_loc = loc;
    let mut value = String::new();
    while c.is_alphabetic() {
        value.push(c);
        next_loc = next_loc.increment(false);
        c = raw[next_loc.index];
    }

    if keyword.contains(&value) {
        Some((
            Token {
                value,
                kind: TokenKind::Keyword,
                location: loc,
            },
            next_loc,
        ))
    } else {
        None
    }
}

fn lex_number(raw: &[char], loc: Location) -> Option<(Token, Location)> {
    let mut c = raw[loc.index];
    let mut next_loc = loc;
    let mut value = String::new();

    while c.is_digit(10) {
        value.push(c);
        next_loc = next_loc.increment(false);
        c = raw[next_loc.index];
    }

    if value.is_empty() {
        None
    } else {
        Some((
            Token {
                value,
                kind: TokenKind::Number,
                location: loc,
            },
            next_loc,
        ))
    }
}

fn lex_operator(raw: &[char], loc: Location) -> Option<(Token, Location)> {
    let operator = ["+", "-", "<"];
    for possible_operator in operator {
        let c = raw[loc.index];
        let next_loc = loc.increment(false);
        // TODO: this won't work with multiple-character syntax bits like >= or ==
        if c.to_string() == possible_operator {
            return Some((
                Token {
                    value: possible_operator.to_string(),
                    kind: TokenKind::Operator,
                    location: loc,
                },
                next_loc,
            ));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexier_oneline_test() {
        let raw = "local n1 = 10;";
        let tokens = match lex(&raw.chars().collect::<Vec<char>>()) {
            Ok(tokens) => tokens,
            Err(msg) => panic!("{}", msg),
        };

        assert_eq!(tokens.len(), 5);
        assert_eq!(
            tokens[0],
            Token {
                value: "local".to_string(),
                kind: TokenKind::Keyword,
                location: Location {
                    index: 0,
                    line: 0,
                    column: 0,
                },
            }
        );
        assert_eq!(
            tokens[1],
            Token {
                value: "n1".to_string(),
                kind: TokenKind::Identifier,
                location: Location {
                    index: 6,
                    line: 0,
                    column: 6,
                },
            }
        );
        assert_eq!(
            tokens[2],
            Token {
                value: "=".to_string(),
                kind: TokenKind::Operator,
                location: Location {
                    index: 9,
                    line: 0,
                    column: 9,
                },
            }
        );
        assert_eq!(
            tokens[3],
            Token {
                value: "10".to_string(),
                kind: TokenKind::Number,
                location: Location {
                    index: 11,
                    line: 0,
                    column: 11,
                },
            }
        );
        assert_eq!(
            tokens[4],
            Token {
                value: ";".to_string(),
                kind: TokenKind::Separator,
                location: Location {
                    index: 13,
                    line: 0,
                    column: 13,
                },
            }
        );
    }

    #[test]
    fn lexier_multiline_test() {
        let raw = "local n1 = 10;\nlocal n2 = 20;";
        let tokens = match lex(&raw.chars().collect::<Vec<char>>()) {
            Ok(tokens) => tokens,
            Err(msg) => panic!("{}", msg),
        };

        assert_eq!(tokens.len(), 10);
        assert_eq!(
            tokens[5],
            Token {
                value: "local".to_string(),
                kind: TokenKind::Keyword,
                location: Location {
                    index: 15,
                    line: 1,
                    column: 0,
                },
            }
        );
        assert_eq!(
            tokens[6],
            Token {
                value: "n2".to_string(),
                kind: TokenKind::Identifier,
                location: Location {
                    index: 21,
                    line: 1,
                    column: 6,
                },
            }
        );
        assert_eq!(
            tokens[8],
            Token {
                value: "20".to_string(),
                kind: TokenKind::Number,
                location: Location {
                    index: 26,
                    line: 1,
                    column: 11,
                },
            }
        );
    }
}
