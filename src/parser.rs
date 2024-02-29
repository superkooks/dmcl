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

        self.decls();
        let s = self.stmts();

        // Replace the current scope with a null one, then set the current scope to the previous one.
        let cur = std::mem::replace(&mut self.cur_scope, symbols::Scope::new(None));
        self.cur_scope = cur.take_prev();

        self.match_tok(Token::C('}'));

        return s;
    }

    fn decls(&mut self) {
        while match self.lookahead {
            Token::Type(_) => true,
            _ => false,
        } {
            let mut t = match self.lookahead.clone() {
                Token::Type(s) => s,
                _ => panic!("unreachable"),
            };

            self.next_tok();
            match self.lookahead {
                Token::Word(_) => (),
                Token::C('[') => {
                    t = DataType::Compound(Box::new(t));
                    self.next_tok();
                    self.match_tok(Token::C(']'))
                }
                _ => panic!("syntax error: decl must have identifier"),
            };

            let ident = ast::Ident::new(self.lookahead.clone(), t, &mut self.prog);
            self.cur_scope.put(self.lookahead.clone(), *ident);

            self.next_tok();
            self.match_tok(Token::C(';'));
        }
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
            Token::C('{') => return self.block(),
            _ => return self.assign(),
        }
    }

    fn assign(&mut self) -> Box<dyn ast::Stmt> {
        match self.lookahead {
            Token::Word(_) => (),
            _ => panic!(
                "syntax error: assignment must have identifier as lhs, found {:?}",
                self.lookahead
            ),
        };

        let id = self.cur_scope.get(self.lookahead.clone()).unwrap();

        self.next_tok();
        match self.lookahead {
            Token::C('[') => {
                self.next_tok();
                let index = self.bool();
                self.match_tok(Token::C(']'));

                self.match_tok(Token::C('='));

                let stmt = Box::new(ast::AssignArray {
                    id: Box::new(id),
                    index,
                    expr: self.bool(),
                });

                self.match_tok(Token::C(';'));

                return stmt;
            }
            Token::C('=') => {
                self.next_tok();

                let stmt = Box::new(ast::Assign {
                    id: Box::new(id),
                    expr: self.bool(),
                });

                self.match_tok(Token::C(';'));

                return stmt;
            }
            _ => panic!(
                "syntax error, unexpected token {:?} when parsing assignment",
                self.lookahead
            ),
        }
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
                self.next_tok();
                let mut array: Vec<Box<dyn ast::Expr>> = Vec::new();

                'outer: loop {
                    array.push(self.bool());

                    match self.lookahead {
                        Token::C(']') => {
                            self.next_tok();
                            break 'outer;
                        }
                        _ => {
                            self.match_tok(Token::C(','));
                        }
                    };
                }

                return Box::new(ast::Array { values: array });
            }
            Token::Integer(i) => {
                let x = Box::new(ast::Const {
                    value: tac::DataVal::Integer(i),
                });
                self.next_tok();
                return x;
            }
            Token::Float(f) => {
                let x = Box::new(ast::Const {
                    value: tac::DataVal::Float(f),
                });
                self.next_tok();
                return x;
            }
            Token::True => {
                let x = Box::new(ast::Const {
                    value: tac::DataVal::Bool(true),
                });
                self.next_tok();
                return x;
            }
            Token::False => {
                let x = Box::new(ast::Const {
                    value: tac::DataVal::Bool(false),
                });
                self.next_tok();
                return x;
            }
            Token::Word(_) => {
                let id = self.cur_scope.get(self.lookahead.clone()).unwrap();
                self.next_tok();

                if self.lookahead == Token::C('[') {
                    self.next_tok();

                    let index = self.bool();
                    self.match_tok(Token::C(']'));

                    return Box::new(ast::ArrayIndex {
                        arr: Box::new(id),
                        index,
                    });
                } else {
                    return Box::new(id);
                }
            }
            _ => panic!("syntax error: token {:?}", self.lookahead),
        }
    }
}
