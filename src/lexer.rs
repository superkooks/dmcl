use core::fmt;
use std::collections::HashMap;

use crate::tac;

pub struct Lexer {
    source: Vec<char>,
    index: usize, // index of that first character we have not parsed
    peek: char,
    line: i64,

    word_table: HashMap<String, Token>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    C(char), // the character itself
    Integer(i64),
    Float(f64),
    Word(String),
    Type(tac::DataType),

    If,
    Else,
    While,
    True,
    False,
    Func,

    BoolOr,
    BoolAnd,
    Eq,
    Ne,
    Le,
    Ge,

    EOF,
}

// We need this for a to_string() method
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Lexer {
    pub fn new(src: Vec<char>) -> Self {
        // Reserve entries in word table
        let mut wt = HashMap::new();
        wt.insert("if".to_string(), Token::If);
        wt.insert("else".to_string(), Token::Else);
        wt.insert("while".to_string(), Token::While);
        wt.insert("true".to_string(), Token::True);
        wt.insert("false".to_string(), Token::False);
        wt.insert("int".to_string(), Token::Type(tac::DataType::Integer));
        wt.insert("float".to_string(), Token::Type(tac::DataType::Float));
        wt.insert("bool".to_string(), Token::Type(tac::DataType::Bool));
        wt.insert("func".to_string(), Token::Func);

        let mut l = Lexer {
            source: src,
            index: 0,
            peek: 0.into(),
            line: 0,
            word_table: wt,
        };
        l.read_char();
        return l;
    }

    fn read_char(&mut self) {
        self.peek = match self.source.get(self.index) {
            Some(c) => *c,
            None => '\x00', // indicates EOF
        };
        self.index += 1;
    }

    fn test_char(&mut self, test: char) -> bool {
        self.read_char();
        if self.peek != test {
            return false;
        } else {
            self.read_char();
            return true;
        }
    }

    pub fn scan(&mut self) -> Token {
        loop {
            if self.peek == ' ' || self.peek == '\t' {
                self.read_char();
            } else if self.peek == '\n' {
                self.read_char();
                self.line += 1
            } else {
                break;
            }
        }

        match self.peek {
            '&' => {
                if self.test_char('&') {
                    return Token::BoolAnd;
                }
            }
            '|' => {
                if self.test_char('|') {
                    return Token::BoolOr;
                }
            }
            '>' => {
                if self.test_char('=') {
                    return Token::Ge;
                } else {
                    return Token::C('>');
                }
            }
            '<' => {
                if self.test_char('=') {
                    return Token::Le;
                } else {
                    return Token::C('<');
                }
            }
            '=' => {
                if self.test_char('=') {
                    return Token::Eq;
                } else {
                    return Token::C('=');
                }
            }
            '!' => {
                if self.test_char('=') {
                    return Token::Ne;
                } else {
                    return Token::C('!');
                }
            }
            '\x00' => {
                return Token::EOF;
            }
            _ => (),
        }

        if self.peek.is_numeric() {
            let mut v: u64 = 0;
            while self.peek.is_numeric() {
                v = 10 * v + self.peek.to_digit(10).unwrap() as u64;
                self.read_char();
            }

            if self.peek != '.' {
                // This is an integer literal
                return Token::Integer(v as i64);
            }

            let mut f = v as f64;
            let mut d = 10.0;
            self.read_char();
            while self.peek.is_numeric() {
                f = f + self.peek.to_digit(10).unwrap() as f64 / d;
                d *= 10.0;
                self.read_char();
            }

            if self.peek != 'f' {
                panic!("syntax error");
            }
            self.read_char();

            return Token::Float(f);
        }

        if self.peek.is_alphabetic() {
            let mut s = String::new();
            while self.peek.is_alphanumeric() {
                s.push(self.peek);
                self.read_char();
            }

            match self.word_table.get(&s) {
                Some(n) => return n.clone(),
                None => {
                    let w = Token::Word(s.clone());
                    self.word_table.insert(s.clone(), w.clone());
                    return w;
                }
            }
        }

        let t = Token::C(self.peek);
        self.read_char();
        return t;
    }
}
