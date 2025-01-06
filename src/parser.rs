use std::collections::HashMap;

use crate::{
    ast::{self, NullStmt},
    lexer::{Lexer, Token},
    scope,
    stac::{self, DataType},
};

pub struct Parser {
    lexer: Lexer,
    lookahead: Token,

    cur_scope: scope::Scope,
    prog: stac::Prog,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Parser {
        let mut p = Parser {
            lexer,
            prog: stac::Prog::new(),
            cur_scope: scope::Scope::new(None),
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

    pub fn program(&mut self) -> &mut stac::Prog {
        let s = self.stmts();
        let mut block = stac::Block::new();
        s.emit(&mut self.prog, &mut block);
        self.prog.entrypoint = self.prog.add_block(block);

        return &mut self.prog;
    }

    fn block(&mut self) -> Box<dyn ast::Stmt> {
        self.match_tok(Token::C('{'));

        // Replace the current scope with a null one, then set the current scope to a new one
        // containing the previous.
        let prev = std::mem::replace(&mut self.cur_scope, scope::Scope::new(None));
        self.cur_scope = scope::Scope::new(Some(Box::new(prev)));

        let s = self.stmts();

        // Replace the current scope with a null one, then set the current scope to the previous one.
        let cur = std::mem::replace(&mut self.cur_scope, scope::Scope::new(None));
        self.cur_scope = cur.take_prev();

        self.match_tok(Token::C('}'));

        return s;
    }

    fn stmts(&mut self) -> Box<dyn ast::Stmt> {
        if self.lookahead == Token::C('}') || self.lookahead == Token::EOF {
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
            Token::Func => {
                self.next_tok();

                match self.lookahead {
                    Token::Extern => {
                        // Extern function
                        self.next_tok();

                        let name = self.lookahead.clone();
                        self.next_tok();

                        self.match_tok(Token::C('('));
                        let params: Vec<stac::DataType> = self
                            .decl_list(Token::C(')'))
                            .iter()
                            .map(|p| {
                                return p.1.clone();
                            })
                            .collect();
                        self.match_tok(Token::C(')'));

                        let returns = self.type_list();

                        // Assign the func to the name
                        self.prog.user_functions.insert(
                            name.clone().into_word().unwrap(),
                            stac::Function {
                                label: stac::Label::CONTINUE,
                                params: params.clone(),
                                returns,
                            },
                        );

                        return Box::new(ast::func::ExternFuncImpl {
                            name: name.into_word().unwrap(),
                            params_count: params.len(),
                        });
                    }
                    _ => {
                        // Regular function

                        let name = self.lookahead.clone();
                        self.next_tok();

                        // Create new scope from previous
                        self.cur_scope = scope::Scope::new(Some(Box::new(std::mem::replace(
                            &mut self.cur_scope,
                            scope::Scope::new(None),
                        ))));

                        // Parse the function signature
                        self.match_tok(Token::C('('));
                        let params: Vec<ast::Ident> = self
                            .decl_list(Token::C(')'))
                            .iter()
                            .map(|p| {
                                let ident = ast::Ident {
                                    name: p.0.clone(),
                                    data_type: p.1.clone(),
                                    addr: self.prog.allocate_var(),
                                };
                                self.cur_scope.put(p.0.clone(), ident.clone());
                                return ident;
                            })
                            .collect();
                        self.match_tok(Token::C(')'));

                        let returns = self.type_list();

                        // Parse the function body
                        let body = self.block();

                        // Create the data type for the function
                        let param_types: Vec<DataType> =
                            params.iter().map(|p| p.data_type.clone()).collect();

                        // pop the func scope
                        self.cur_scope =
                            std::mem::replace(&mut self.cur_scope, scope::Scope::new(None))
                                .take_prev();

                        // Assign the func to the name
                        self.prog.user_functions.insert(
                            name.clone().into_word().unwrap(),
                            stac::Function {
                                label: stac::Label::CONTINUE,
                                params: param_types,
                                returns,
                            },
                        );

                        // Return the function
                        return Box::new(ast::func::FuncImpl {
                            name: name.into_word().unwrap(),
                            body,
                            params,
                        });
                    }
                }
            }
            Token::Return => {
                self.next_tok();

                // Collect the parameters
                let mut values = vec![];
                while self.lookahead.clone() != Token::C(';') {
                    values.push(self.bool());
                }
                self.next_tok();

                return Box::new(ast::func::Return { values });
            }
            Token::Struct => {
                self.next_tok();
                let name = self.lookahead.clone();
                self.next_tok();

                self.match_tok(Token::C('{'));
                let fields = self.decl_list(Token::C('}'));
                self.match_tok(Token::C('}'));

                let mut types = vec![];
                let mut names = HashMap::new();
                for (idx, field) in fields.iter().enumerate() {
                    types.push(field.1.clone());
                    names.insert(field.0.clone().into_word().unwrap(), idx);
                }

                self.prog
                    .user_structs
                    .insert(name.into_word().unwrap(), stac::Struct { types, names });

                return Box::new(ast::NullStmt {});
            }
            Token::C('{') => return self.block(),
            _ => return self.assign(),
        }
    }

    // Caller is responsible for the start and end token ()/[]
    fn bool_list(&mut self, end_tok: Token) -> Vec<Box<dyn ast::Expr>> {
        let mut list = vec![];

        while self.lookahead != end_tok {
            if self.lookahead == Token::C(',') {
                self.next_tok();
            }

            list.push(self.bool());
        }

        return list;
    }

    // Caller is responsible for the start and end token ()/[]
    fn decl_list(&mut self, end_tok: Token) -> Vec<(Token, DataType)> {
        let mut list = Vec::new();

        while self.lookahead != end_tok {
            if self.lookahead == Token::C(',') {
                self.next_tok();
            }

            let name = match self.lookahead.clone() {
                Token::Word(w) => Token::Word(w),
                _ => panic!("syntax error: decl must have identifier"),
            };
            self.next_tok();
            self.match_tok(Token::C(':'));

            let data_type = match self.lookahead.clone() {
                Token::Type(s) => s,
                _ => panic!("syntax error: decl must have a type"),
            };
            self.next_tok();

            list.push((name, data_type));
        }

        return list;
    }

    fn type_list(&mut self) -> Vec<DataType> {
        self.match_tok(Token::C('('));

        let mut list = Vec::new();

        while self.lookahead != Token::C(')') {
            if self.lookahead == Token::C(',') {
                self.next_tok();
            }

            let data_type = match self.lookahead.clone() {
                Token::Type(s) => s,
                _ => panic!("syntax error: must have a type"),
            };
            list.push(data_type);

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
                    data_type: expr.out_type(&self.prog),
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
                // Function call (returns ignored)
                self.next_tok();
                let params = self.bool_list(Token::C(')'));
                self.next_tok();

                stmt = Box::new(ast::func::FuncCall {
                    func: id_tok.into_word().unwrap(),
                    params,
                });
            }
            Token::C('[') => {
                // Array index
                self.next_tok();
                let index = self.bool();
                self.match_tok(Token::C(']'));

                self.match_tok(Token::C('='));

                let id = self
                    .cur_scope
                    .get(id_tok.clone())
                    .expect(&format!("unknown identifier: {}", id_tok));

                let stmt = Box::new(ast::compound::AssignArray {
                    id: id,
                    index,
                    expr: self.bool(),
                });

                self.match_tok(Token::C(';'));

                return stmt;
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
            return self.field();
        }
    }

    fn field(&mut self) -> Box<dyn ast::Expr> {
        let x = self.factor();
        if self.lookahead == Token::C('.') {
            self.next_tok();
            let field = self.lookahead.clone().into_word().unwrap();
            self.next_tok();
            return Box::new(ast::compound::StructAccess { expr: x, field });
        } else {
            return x;
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
                // Array literal
                self.next_tok();
                let array: Vec<Box<dyn ast::Expr>> = self.bool_list(Token::C(']'));
                self.next_tok();

                return Box::new(ast::compound::ArrayLiteral { values: array });
            }
            Token::String(s) => {
                // String literal
                self.next_tok();
                return Box::new(ast::Const {
                    value: stac::DataVal::String(s),
                    data_type: DataType::String,
                });
            }
            Token::Integer(i) => {
                let x = Box::new(ast::Const {
                    value: stac::DataVal::Integer(i),
                    data_type: stac::DataType::Integer,
                });
                self.next_tok();
                return x;
            }
            Token::Float(f) => {
                let x = Box::new(ast::Const {
                    value: stac::DataVal::Float(f),
                    data_type: stac::DataType::Float,
                });
                self.next_tok();
                return x;
            }
            Token::True => {
                let x = Box::new(ast::Const {
                    value: stac::DataVal::Bool(true),
                    data_type: stac::DataType::Bool,
                });
                self.next_tok();
                return x;
            }
            Token::False => {
                let x = Box::new(ast::Const {
                    value: stac::DataVal::Bool(false),
                    data_type: stac::DataType::Bool,
                });
                self.next_tok();
                return x;
            }
            Token::Word(_) => {
                let id_tok = self.lookahead.clone();
                let id = self.cur_scope.get(id_tok.clone());
                self.next_tok();

                if self.lookahead == Token::C('[') {
                    // Array index
                    self.next_tok();

                    let index = self.bool();
                    self.match_tok(Token::C(']'));

                    return Box::new(ast::compound::ArrayIndex {
                        arr: Box::new(id.unwrap()),
                        index,
                    });
                } else if self.lookahead == Token::C('(') {
                    // Function call as an expression
                    self.next_tok();
                    let params: Vec<Box<dyn ast::Expr>> = self.bool_list(Token::C(')'));
                    self.next_tok();

                    return Box::new(ast::func::FuncCall {
                        func: id_tok.into_word().unwrap(),
                        params,
                    });
                } else if self.lookahead == Token::C('{') {
                    // Struct literal
                    self.next_tok();
                    let mut list = vec![];

                    while self.lookahead != Token::C('}') {
                        if self.lookahead == Token::C(',') {
                            self.next_tok();
                        }

                        let name = self.lookahead.clone().into_word().unwrap();
                        self.next_tok();
                        self.match_tok(Token::C(':'));

                        let value = self.bool();

                        list.push((name, value));
                    }
                    self.next_tok();

                    return Box::new(ast::compound::StructLiteral {
                        strct: id_tok.into_word().unwrap(),
                        values: list,
                    });
                } else {
                    return Box::new(id.unwrap());
                }
            }
            _ => panic!("syntax error: token {:?}", self.lookahead),
        }
    }
}
