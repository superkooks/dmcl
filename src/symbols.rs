use std::collections::HashMap;

use crate::{ast, lexer};

pub struct Scope {
    pub prev: Option<Box<Scope>>,
    sym_table: HashMap<String, ast::Ident>,
}

impl<'a> Scope {
    pub fn new(prev: Option<Box<Scope>>) -> Scope {
        return Scope {
            prev,
            sym_table: HashMap::new(),
        };
    }

    pub fn put(&mut self, w: lexer::Token, i: ast::Ident) {
        match w {
            lexer::Token::Word(s) => self.sym_table.insert(s, i),
            _ => panic!("cannot save non-word in symbol table: {:?}", w),
        };
    }

    pub fn get(&self, w: lexer::Token) -> Option<ast::Ident> {
        let s = match w {
            lexer::Token::Word(ref s) => s,
            _ => panic!("cannot get non-word from symbol table: {:?}", w),
        };
        let found = self.sym_table.get(s);

        match found {
            Some(_) => return found.cloned(),
            None => {
                return match &self.prev {
                    Some(s) => s.get(w),
                    None => None,
                }
            }
        }
    }

    pub fn take_prev(self) -> Scope {
        return *self.prev.unwrap();
    }
}
