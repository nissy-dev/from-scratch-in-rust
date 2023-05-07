#[derive(Debug)]
pub struct StyleSheet {
    rules: Vec<Rule>,
}

#[derive(Debug)]
struct Rule {
    selectors: Vec<Selector>,
    declarations: Vec<Declaration>,
}

#[derive(Debug)]
enum Selector {
    Simple(SimpleSelector),
}

#[derive(Debug)]
struct SimpleSelector {
    tag_name: Option<String>,
    id: Option<String>,
    class: Vec<String>,
}

#[derive(Debug)]
struct Declaration {
    name: String,
    value: Value,
}

#[derive(Debug)]
enum Value {
    Keyword(String),
    Length(f32, Unit),
    ColorValue(Color),
}

#[derive(Debug)]
enum Unit {
    Px,
}

#[derive(Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

type Specificity = (usize, usize, usize);

impl Selector {
    fn specificity(&self) -> Specificity {
        // https://www.w3.org/TR/selectors/#specificity
        let Selector::Simple(ref simple) = *self;
        let a = simple.id.iter().count();
        let b = simple.class.len();
        let c = simple.tag_name.iter().count();
        (a, b, c)
    }
}

struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    fn next_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn consume_char(&mut self) -> char {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, current_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        current_char
    }

    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char())
        }
        result
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    // This is a really simple identifier parser, but it's good enough for our purposes.
    fn parse_identifier(&mut self) -> String {
        self.consume_while(|c| match c {
            'a'..='z' | '-' => true,
            _ => false,
        })
    }

    fn parse_simple_selector(&mut self) -> SimpleSelector {
        let mut simple_selector = SimpleSelector {
            tag_name: None,
            id: None,
            class: Vec::new(),
        };
        while !self.eof() {
            match self.next_char() {
                '#' => {
                    // ID selector
                    self.consume_char();
                    simple_selector.id = Some(self.parse_identifier());
                }
                '.' => {
                    // Class selector
                    self.consume_char();
                    simple_selector.class.push(self.parse_identifier());
                }
                '*' => {
                    // Universal selector
                    self.consume_char();
                }
                c if matches!(c, 'a'..='z') => {
                    // Type selector
                    simple_selector.tag_name = Some(self.parse_identifier());
                }
                _ => break,
            }
        }

        simple_selector
    }

    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            selectors.push(Selector::Simple(self.parse_simple_selector()));
            self.consume_whitespace();
            match self.next_char() {
                ',' => {
                    self.consume_char();
                    self.consume_whitespace();
                }
                '{' => break,
                c => panic!("Unexpected character {} in selector list", c),
            }
        }
        selectors.sort_by(|a, b| b.specificity().cmp(&a.specificity()));
        selectors
    }

    fn parse_length(&mut self) -> Value {
        let length_str = self.consume_while(|c| matches!(c, '0'..='9' | '.'));
        let length = length_str.parse::<f32>().unwrap();
        let unit_str = self.consume_while(|c| matches!(c, 'a'..='z'));
        let unit = match &*unit_str {
            "px" => Unit::Px,
            _ => panic!("unexpected unit: {}", &unit_str),
        };
        Value::Length(length, unit)
    }

    fn parse_hex_pair(&mut self) -> u8 {
        let s = &self.input[self.pos..self.pos + 2];
        self.pos += 2;
        u8::from_str_radix(&s.to_lowercase(), 16).unwrap()
    }

    fn parse_color(&mut self) -> Value {
        assert!(self.consume_char() == '#');
        Value::ColorValue(Color {
            r: self.parse_hex_pair(),
            g: self.parse_hex_pair(),
            b: self.parse_hex_pair(),
            a: 255, // 1.0 opaque
        })
    }

    fn parse_value(&mut self) -> Value {
        match self.next_char() {
            '0'..='9' => self.parse_length(),
            '#' => self.parse_color(),
            _ => Value::Keyword(self.parse_identifier()),
        }
    }

    fn parse_declaration(&mut self) -> Declaration {
        let name = self.parse_identifier();
        self.consume_whitespace();
        assert!(self.consume_char() == ':');
        self.consume_whitespace();
        let value = self.parse_value();
        self.consume_whitespace();
        assert!(self.consume_char() == ';');
        Declaration { name, value }
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        assert!(self.consume_char() == '{');
        let mut declarations = Vec::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == '}' {
                self.consume_char();
                break;
            }
            declarations.push(self.parse_declaration());
        }
        declarations
    }

    fn parse_rule(&mut self) -> Rule {
        Rule {
            selectors: self.parse_selectors(),
            declarations: self.parse_declarations(),
        }
    }

    fn parse_rules(&mut self) -> Vec<Rule> {
        let mut rules = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }
            rules.push(self.parse_rule());
        }
        rules
    }
}

pub fn parse(source: String) -> StyleSheet {
    let mut parser = Parser {
        pos: 0,
        input: source,
    };

    let rules = parser.parse_rules();
    StyleSheet { rules }
}
