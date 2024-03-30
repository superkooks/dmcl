use crate::{
    ast::{self, NullStmt},
    lexer::{Lexer, Token},
    symbols,
    tac::{self, DataType},
};

pub struct Parser {
    lexer: Lexer,
    lookahead: Token,

    cur_scope: symbols::Scope,
    prog: tac::Prog,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Parser {
        let mut p = Parser {
            lexer,
            prog: tac::Prog::new(),
            cur_scope: symbols::Scope::new(None),
            lookahead: Token::C(' '),
        };
        p.next_tok();
        return p;
    }

    fn match_tok(&mut self, t: Token) {
        if self.lookahead == t {
            self.next_tok();
        } else {
            panic!(
                "syntax error: next token didn't match: {:?} where {:?} expected",
                self.lookahead, t,
            );
        }
    }

    fn next_tok(&mut self) {
        self.lookahead = self.lexer.scan();
    }

    pub fn program(&mut self) -> &mut tac::Prog {
        let s = self.block();
        s.emit(&mut self.prog);

        return &mut self.prog;
    }

    fn block(&mut self) -> Box<dyn ast::Stmt> {
        self.match_tok(Token::C('{'));

        // Replace the current scope with a null one, then set the current scope to a new one
        // containing the previous.
        let prev = std::mem::replace(&mut self.cur_scope, symbols::Scope::new(None));
        self.cur_scope = symbols::Scope::new(Some(Box::new(prev)));

        let s = self.stmts();

        // Replace the current scope with a null one, then set the current scope to the previous one.
        let cur = std::mem::replace(&mut self.cur_scope, symbols::Scope::new(None));
        self.cur_scope = cur.take_prev();

        self.match_tok(Token::C('}'));

        return s;
    }

    fn stmts(&mut self) -> Box<dyn ast::Stmt> {
        if self.lookahead == Token::C('}') {
            return Box::new(ast::NullStmt {});
        } else {
            return Box::new(ast::Seq {
                stmt1: self.stmt(),
                stmt2: self.stmts(),
            });
        }
    }

    fn stmt(&mut self) -> Box<dyn ast::Stmt> {
        match self.lookahead {
            Token::C(';') => {
                self.next_tok();
                return Box::new(NullStmt {});
            }
            Token::If => {
                self.next_tok();
                let e = self.bool();
                let s_t = self.block();
                if self.lookahead != Token::Else {
                    return Box::new(ast::If { expr: e, stmt: s_t });
                }

                self.next_tok();
                let s_f = self.block();
                return Box::new(ast::IfElse {
                    expr: e,
                    stmt_t: s_t,
                    stmt_f: s_f,
                });
            }
            Token::While => {
                self.next_tok();
                let e = self.bool();
                let body = self.block();
                return Box::new(ast::While {
                    expr: e,
                    stmt: body,
                });
            }
            // Token::Func => {
            //     self.next_tok();

            //     let name = self.lookahead.clone();
            //     self.next_tok();

            //     // Parse the function signature
            //     let params: Vec<Ident> = self
            //         .decl_list()
            //         .iter()
            //         .map(|p| {
            //             let ident = ast::Ident::new(p.0.clone(), p.1.clone(), &mut self.prog);
            //             self.cur_scope.put(p.0.clone(), *ident.clone());
            //             return *ident;
            //         })
            //         .collect();

            //     let returns: Vec<Ident> = self
            //         .decl_list()
            //         .iter()
            //         .map(|p| {
            //             let ident = ast::Ident::new(p.0.clone(), p.1.clone(), &mut self.prog);
            //             self.cur_scope.put(p.0.clone(), *ident.clone());
            //             return *ident;
            //         })
            //         .collect();

            //     // Parse the function body
            //     let body = self.block();

            //     // Create the data type for the function
            //     let params_types: Vec<DataType> =
            //         params.iter().map(|p| p.data_type.clone()).collect();
            //     let returns_types: Vec<DataType> =
            //         returns.iter().map(|p| p.data_type.clone()).collect();

            //     let name_ident = ast::Ident::new(
            //         name.clone(),
            //         DataType::Func {
            //             params: params_types.clone(),
            //             returns: returns_types.clone(),
            //         },
            //         &mut self.prog,
            //     );

            //     // Return the function
            //     let params_mem = params.iter().map(|p| p.addr).collect();
            //     let returns_mem = returns.iter().map(|p| p.addr).collect();

            //     return Box::new(FuncImpl {
            //         name,
            //         name_addr: name_ident.addr,
            //         body,
            //         params: params_types,
            //         returns: returns_types,
            //         params_mem,
            //         returns_mem,
            //     });
            // }
            Token::C('{') => return self.block(),
            _ => return self.assign(),
        }
    }

    fn decl_list(&mut self) -> Vec<(Token, DataType)> {
        self.match_tok(Token::C('('));

        let mut list = Vec::new();

        while self.lookahead != Token::C(')') {
            if self.lookahead == Token::C(',') {
                self.next_tok();
            }

            let name = match self.lookahead.clone() {
                Token::Word(w) => Token::Word(w),
                _ => panic!("syntax error: decl must have identifier"),
            };

            self.next_tok();
            let data_type = match self.lookahead.clone() {
                Token::Type(s) => s,
                _ => panic!("syntax error: decl must have a type"),
            };

            list.push((name, data_type));

            self.next_tok();
        }
        self.next_tok();

        return list;
    }

    fn assign(&mut self) -> Box<dyn ast::Stmt> {
        match self.lookahead {
            Token::Word(_) => (),
            _ => panic!(
                "syntax error: assignment must have identifier as lhs, found {:?}",
                self.lookahead
            ),
        };

        let id_tok = self.lookahead.clone();

        self.next_tok();

        let stmt: Box<dyn ast::Stmt>;
        match self.lookahead {
            Token::DeclAssign => {
                // Declare and assign
                self.next_tok();
                let expr = self.bool();

                let id = ast::Ident {
                    addr: self.prog.allocate_var(),
                    name: id_tok.clone(),
                    data_type: expr.out_type(),
                };

                self.cur_scope.put(id_tok, id.clone());

                stmt = Box::new(ast::Assign { id, expr })
            }
            Token::C('=') => {
                // Assignment
                self.next_tok();
                let id = self
                    .cur_scope
                    .get(id_tok.clone())
                    .expect(&format!("unknown identifier: {}", id_tok));

                stmt = Box::new(ast::Assign {
                    id: id,
                    expr: self.bool(),
                });
            }
            Token::C('(') => {
                // Function call

                // // Parse function parameters
                // self.next_tok();
                // let mut params: Vec<Box<dyn ast::Expr>> = Vec::new();

                // 'outer: loop {
                //     params.push(self.bool());

                //     match self.lookahead {
                //         Token::C(')') => {
                //             self.next_tok();
                //             break 'outer;
                //         }
                //         _ => {
                //             self.match_tok(Token::C(','));
                //         }
                //     };
                // }

                // stmt = Box::new(ast::FuncCall {
                //     func: Box::new(id),
                //     params,
                // });
                unimplemented!()
            }
            Token::C('[') => {
                // self.next_tok();
                // let index = self.bool();
                // self.match_tok(Token::C(']'));

                // self.match_tok(Token::C('='));

                // let stmt = Box::new(ast::AssignArray {
                //     id: Box::new(id),
                //     index,
                //     expr: self.bool(),
                // });

                // self.match_tok(Token::C(';'));

                // return stmt;
                unimplemented!()
            }
            _ => panic!("unknown statement"),
        }

        self.match_tok(Token::C(';'));

        return stmt;
    }

    // This part specifies the order of operations through the heirarchy
    fn bool(&mut self) -> Box<dyn ast::Expr> {
        let mut x = self.join();
        while self.lookahead == Token::BoolOr {
            self.next_tok();
            x = Box::new(ast::BoolOr { x, y: self.join() });
        }
        return x;
    }

    fn join(&mut self) -> Box<dyn ast::Expr> {
        let mut x = self.equality();
        while self.lookahead == Token::BoolAnd {
            self.next_tok();
            x = Box::new(ast::BoolAnd {
                x,
                y: self.equality(),
            });
        }
        return x;
    }

    fn equality(&mut self) -> Box<dyn ast::Expr> {
        let mut x = self.rel();
        while self.lookahead == Token::Eq || self.lookahead == Token::Ne {
            let tok = self.lookahead.clone();
            self.next_tok();
            x = Box::new(ast::Arith {
                op: tok,
                x,
                y: self.rel(),
            });
        }
        return x;
    }

    fn rel(&mut self) -> Box<dyn ast::Expr> {
        let mut x = self.expr();
        while match self.lookahead {
            Token::Ge | Token::Le | Token::C('<') | Token::C('>') => true,
            _ => false,
        } {
            let tok = self.lookahead.clone();
            self.next_tok();
            x = Box::new(ast::Arith {
                op: tok,
                x,
                y: self.expr(),
            });
        }
        return x;
    }

    fn expr(&mut self) -> Box<dyn ast::Expr> {
        let mut x = self.term();
        while self.lookahead == Token::C('-') || self.lookahead == Token::C('+') {
            let tok = self.lookahead.clone();
            self.next_tok();
            x = Box::new(ast::Arith {
                op: tok,
                x,
                y: self.term(),
            });
        }
        return x;
    }

    fn term(&mut self) -> Box<dyn ast::Expr> {
        let mut x = self.unary();
        while self.lookahead == Token::C('*') || self.lookahead == Token::C('/') {
            let tok = self.lookahead.clone();
            self.next_tok();
            x = Box::new(ast::Arith {
                op: tok,
                x,
                y: self.unary(),
            });
        }
        return x;
    }

    fn unary(&mut self) -> Box<dyn ast::Expr> {
        if self.lookahead == Token::C('-') {
            self.next_tok();
            return Box::new(ast::Unary {
                op: self.lookahead.clone(),
                x: self.unary(),
            });
        } else if self.lookahead == Token::C('!') {
            self.next_tok();
            return Box::new(ast::BoolNot { x: self.unary() });
        } else {
            return self.factor();
        }
    }

    fn factor(&mut self) -> Box<dyn ast::Expr> {
        match self.lookahead.clone() {
            Token::C('(') => {
                self.next_tok();
                let x = self.bool();
                self.match_tok(Token::C(')'));
                return x;
            }
            Token::C('[') => {
                // Array immediate
                // self.next_tok();
                // let mut array: Vec<Box<dyn ast::Expr>> = Vec::new();

                // 'outer: loop {
                //     array.push(self.bool());

                //     match self.lookahead {
                //         Token::C(']') => {
                //             self.next_tok();
                //             break 'outer;
                //         }
                //         _ => {
                //             self.match_tok(Token::C(','));
                //         }
                //     };
                // }

                // return Box::new(ast::Array { values: array });
                panic!("oof");
            }
            Token::Integer(i) => {
                let x = Box::new(ast::Const {
                    value: tac::DataVal::Integer(i),
                    data_type: tac::DataType::Integer,
                });
                self.next_tok();
                return x;
            }
            Token::Float(f) => {
                let x = Box::new(ast::Const {
                    value: tac::DataVal::Float(f),
                    data_type: tac::DataType::Float,
                });
                self.next_tok();
                return x;
            }
            Token::True => {
                let x = Box::new(ast::Const {
                    value: tac::DataVal::Bool(true),
                    data_type: tac::DataType::Bool,
                });
                self.next_tok();
                return x;
            }
            Token::False => {
                let x = Box::new(ast::Const {
                    value: tac::DataVal::Bool(false),
                    data_type: tac::DataType::Bool,
                });
                self.next_tok();
                return x;
            }
            Token::Word(_) => {
                let id = self.cur_scope.get(self.lookahead.clone()).unwrap();
                self.next_tok();

                if self.lookahead == Token::C('[') {
                    // self.next_tok();

                    // let index = self.bool();
                    // self.match_tok(Token::C(']'));

                    // return Box::new(ast::ArrayIndex {
                    //     arr: Box::new(id),
                    //     index,
                    // });
                    panic!("oof");
                } else {
                    return Box::new(id);
                }
            }
            _ => panic!("syntax error: token {:?}", self.lookahead),
        }
    }
}
