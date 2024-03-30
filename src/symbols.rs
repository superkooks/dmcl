use std::collections::HashMap;

use crate::{ast, lexer, tac};

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

    // Find the ident in this scope, or assign a new identifier
    // pub fn assign(
    //     &mut self,
    //     w: lexer::Token,
    //     d: tac::DataType,
    //     prog: &mut tac::Prog,
    // ) -> ast::Ident {
    //     let s = match w.clone() {
    //         lexer::Token::Word(s) => s,
    //         _ => panic!("cannot get non-word from symbol table: {:?}", w),
    //     };
    //     let found = self.sym_table.get(&s);

    //     match found {
    //         Some(_) => found.cloned().unwrap(),
    //         None => {
    //             let a = prog.allocate_var();
    //             let i = ast::Ident {
    //                 data_type: d,
    //                 name: w,
    //                 addr: a,
    //             };
    //             self.sym_table.insert(s, i.clone());
    //             return i;
    //         }
    //     }
    // }

    // Find the ident by searching expanding scopes
    pub fn get(&self, w: lexer::Token) -> Option<ast::Ident> {
        let s = match w.clone() {
            lexer::Token::Word(s) => s,
            _ => panic!("cannot get non-word from symbol table: {:?}", w),
        };
        let found = self.sym_table.get(&s);

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
