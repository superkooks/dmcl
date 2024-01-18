use std::collections::HashMap;

use crate::{lexer, tac};

struct Scope<'a> {
    prev: &'a Scope<'a>,
    sym_table: HashMap<String, tac::Ident>,
}

impl Scope<'_> {
    fn put(&mut self, w: lexer::Token, i: tac::Ident) {
        match w {
            lexer::Token::Word(s) => self.sym_table.insert(s, i),
            _ => panic!("cannot save non-word in symbol table"),
        };
    }

    fn get(&self, w: lexer::Token) -> Option<&tac::Ident> {
        let s = match w {
            lexer::Token::Word(ref s) => s,
            _ => panic!("cannot save non-word in symbol table"),
        };
        let found = self.sym_table.get(s);

        match found {
            Some(_) => return found,
            None => return self.prev.get(w),
        }
    }
}
