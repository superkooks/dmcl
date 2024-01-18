use core::fmt;
use std::collections::HashMap;

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
    Integer(u64),
    Float(f64),
    Word(String),

    Assignment,
    If,

    True,
    False,

    BoolOr,
    BoolAnd,
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
        wt.insert("true".to_string(), Token::True);
        wt.insert("false".to_string(), Token::False);

        Lexer {
            source: src,
            index: 0,
            peek: 0.into(),
            line: 0,
            word_table: wt,
        }
    }

    fn read_char(&mut self) {
        self.peek = self.source[self.index];
        self.index += 1;
    }

    fn test_char(&mut self, test: char) -> bool {
        self.read_char();
        if self.peek != test {
            return false;
        } else {
            return true;
        }
    }

    pub fn scan(&mut self) -> Token {
        loop {
            self.read_char();
            if self.peek == ' ' || self.peek == '\t' {
                continue;
            } else if self.peek == '\n' {
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

            '=' => return Token::Assignment,
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
                return Token::Integer(v);
            }

            let mut f = v as f64;
            let mut d = 10.0;
            self.read_char();
            while self.peek.is_numeric() {
                f = f + d / self.peek.to_digit(10).unwrap() as f64;
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

        return Token::C(self.peek);
    }
}
