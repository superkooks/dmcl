use crate::{
    ast::{self, NullStmt},
    lexer::{Lexer, Token},
    symbols, tac,
};

pub struct Parser<'a> {
    lexer: Lexer,
    lookahead: Token,

    top_table: symbols::Scope<'a>,
    prog: tac::Prog,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer) -> Parser<'a> {
        let mut p = Parser {
            lexer,
            prog: tac::Prog::new(),
            top_table: symbols::Scope::new(None),
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
        self.decls();
        let s = self.stmts();
        self.match_tok(Token::C('}'));

        return s;
    }

    fn decls(&mut self) {
        while match self.lookahead {
            Token::Type(_) => true,
            _ => false,
        } {
            let t = match self.lookahead {
                Token::Type(s) => s,
                _ => panic!("unreachable"),
            };

            self.next_tok();
            match self.lookahead {
                Token::Word(_) => (),
                _ => panic!("syntax error: decl must have identifier"),
            };

            let ident = ast::Ident::new(self.lookahead.clone(), t, &mut self.prog);
            self.top_table.put(self.lookahead.clone(), *ident);

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

    fn assign(&mut self) -> Box<ast::Assign> {
        match self.lookahead {
            Token::Word(_) => (),
            _ => panic!("syntax error: assignment must have identifier as lhs"),
        };

        let id = self.top_table.get(self.lookahead.clone()).unwrap();

        self.next_tok();
        self.match_tok(Token::C('='));

        let stmt = Box::new(ast::Assign {
            id,
            expr: self.bool(),
        });

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
            Token::Integer(_) => {
                let x = Box::new(ast::Const {
                    token: self.lookahead.clone(),
                    data_type: tac::DataType::Integer(0),
                });
                self.next_tok();
                return x;
            }
            Token::Float(_) => {
                let x = Box::new(ast::Const {
                    token: self.lookahead.clone(),
                    data_type: tac::DataType::Float(0.0),
                });
                self.next_tok();
                return x;
            }
            Token::True => {
                let x = Box::new(ast::Const {
                    token: self.lookahead.clone(),
                    data_type: tac::DataType::Bool(false),
                });
                self.next_tok();
                return x;
            }
            Token::False => {
                let x = Box::new(ast::Const {
                    token: self.lookahead.clone(),
                    data_type: tac::DataType::Bool(false),
                });
                self.next_tok();
                return x;
            }
            Token::Word(_) => {
                let id = self.top_table.get(self.lookahead.clone()).unwrap();
                self.next_tok();
                return Box::new(id);
            }
            _ => panic!("syntax error: token {:?}", self.lookahead),
        }
    }
}
