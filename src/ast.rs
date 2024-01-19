use crate::{
    lexer,
    tac::{self, DataType},
};

pub trait Expr {
    // Resolve the expression, potentially adding instructions to the program,
    // returning the address where the value is stored
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr;

    fn in_type(&self) -> DataType;
    fn out_type(&self) -> DataType;
}

#[derive(Clone)]
pub struct Ident {
    name: lexer::Token,
    addr: tac::Addr,
    data_type: DataType,
}

impl Ident {
    pub fn new(name: lexer::Token, data_type: DataType, prog: &mut tac::Prog) -> Box<Self> {
        return Box::new(Ident {
            name,
            data_type,
            addr: prog.allocate_var(),
        });
    }
}

impl Expr for Ident {
    fn emit(self: Box<Self>, _: &mut tac::Prog) -> tac::Addr {
        return self.addr;
    }

    fn in_type(&self) -> DataType {
        panic!("no in type for ident")
    }

    fn out_type(&self) -> DataType {
        return self.data_type;
    }
}

pub struct Arith {
    pub op: lexer::Token,
    pub x: Box<dyn Expr>,
    pub y: Box<dyn Expr>,
}

impl Expr for Arith {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let x_res = self.x.emit(prog);
        let y_res = self.y.emit(prog);

        // Create temp address to store result
        let temp = prog.allocate_var();
        prog.add_instr(tac::Instr::AssignExpr {
            op: self.op,
            to: temp,
            x: x_res,
            y: y_res,
        });
        return temp;
    }

    fn in_type(&self) -> DataType {
        let x = self.x.out_type();
        let y = self.y.out_type();
        if x == y {
            return x;
        } else {
            panic!("type error: inputs to arith have different types")
        }
    }

    fn out_type(&self) -> DataType {
        use lexer::Token;
        match self.op {
            Token::Eq | Token::Ne | Token::Le | Token::Ge | Token::C('>') | Token::C('<') => {
                return DataType::Bool(false)
            }
            _ => return self.in_type(),
        }
    }
}

pub struct Unary {
    pub op: lexer::Token,
    pub x: Box<dyn Expr>,
}

impl Expr for Unary {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let x_res = self.x.emit(prog);
        let temp = prog.allocate_var();
        prog.add_instr(tac::Instr::AssignExpr {
            op: self.op,
            to: temp,
            x: x_res,
            y: tac::Addr(0),
        });
        return temp;
    }

    fn in_type(&self) -> DataType {
        return self.x.out_type();
    }

    fn out_type(&self) -> DataType {
        return self.in_type();
    }
}

pub struct Const {
    pub token: lexer::Token,
    pub data_type: DataType,
}

impl Expr for Const {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let temp = prog.allocate_var();
        prog.add_instr(tac::Instr::StoreConst {
            c: self.token,
            addr: temp,
        });
        return temp;
    }

    fn in_type(&self) -> DataType {
        panic!("no in type for ident")
    }

    fn out_type(&self) -> DataType {
        return self.data_type;
    }
}

pub struct BoolOr {
    pub x: Box<dyn Expr>,
    pub y: Box<dyn Expr>,
}

impl Expr for BoolOr {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let x_res = self.x.emit(prog);

        // Lazy evaluate the second operand
        let if1 = prog.add_temp_instr();

        let y_res = self.y.emit(prog);

        let if2 = prog.add_temp_instr();

        // Create true and false branches
        let output = prog.allocate_var();
        prog.add_instr(tac::Instr::StoreConst {
            c: lexer::Token::False,
            addr: output,
        });

        let goto = prog.add_temp_instr();

        let t_branch = prog.add_instr(tac::Instr::StoreConst {
            c: lexer::Token::True,
            addr: output,
        });

        // Change the labels of the if/goto statements
        prog.mod_instr(
            if1,
            tac::Instr::IfExpr {
                test: x_res,
                if_true: t_branch,
                if_false: tac::Label::CONTINUE,
            },
        );
        prog.mod_instr(
            if2,
            tac::Instr::IfExpr {
                test: y_res,
                if_true: t_branch,
                if_false: tac::Label::CONTINUE,
            },
        );
        prog.mod_instr(
            goto,
            tac::Instr::Goto {
                label: t_branch.next(),
            },
        );

        return output;
    }

    fn in_type(&self) -> DataType {
        return DataType::Bool(false);
    }

    fn out_type(&self) -> DataType {
        return DataType::Bool(false);
    }
}

pub struct BoolAnd {
    pub x: Box<dyn Expr>,
    pub y: Box<dyn Expr>,
}

impl Expr for BoolAnd {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let x_res = self.x.emit(prog);

        // Lazy evaluate the second operand
        let if1 = prog.add_temp_instr();

        let y_res = self.y.emit(prog);

        let if2 = prog.add_temp_instr();

        // Create true and false branches
        let output = prog.allocate_var();
        prog.add_instr(tac::Instr::StoreConst {
            c: lexer::Token::True,
            addr: output,
        });

        let goto = prog.add_temp_instr();

        let f_branch = prog.add_instr(tac::Instr::StoreConst {
            c: lexer::Token::False,
            addr: output,
        });

        // Change the labels of if/goto statements
        prog.mod_instr(
            if1,
            tac::Instr::IfExpr {
                test: x_res,
                if_true: tac::Label::CONTINUE,
                if_false: f_branch,
            },
        );
        prog.mod_instr(
            if2,
            tac::Instr::IfExpr {
                test: y_res,
                if_true: tac::Label::CONTINUE,
                if_false: f_branch,
            },
        );
        prog.mod_instr(
            goto,
            tac::Instr::Goto {
                label: f_branch.next(),
            },
        );

        return output;
    }

    fn in_type(&self) -> DataType {
        return DataType::Bool(false);
    }

    fn out_type(&self) -> DataType {
        return DataType::Bool(false);
    }
}

pub struct BoolNot {
    pub x: Box<dyn Expr>,
}

impl Expr for BoolNot {
    // Implement not using jumps instead of a dedicated op in AssignExpr, i guess...
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let x_res = self.x.emit(prog);

        // Lazy evaluate the second operand
        let if1 = prog.add_temp_instr();

        // Create true and false branches
        let output = prog.allocate_var();
        prog.add_instr(tac::Instr::StoreConst {
            c: lexer::Token::False,
            addr: output,
        });

        let goto = prog.add_temp_instr();

        let f_branch = prog.add_instr(tac::Instr::StoreConst {
            c: lexer::Token::True, // false branch returns true
            addr: output,
        });

        // Change the labels of if/goto statements
        prog.mod_instr(
            if1,
            tac::Instr::IfExpr {
                test: x_res,
                if_true: tac::Label::CONTINUE,
                if_false: f_branch,
            },
        );
        prog.mod_instr(
            goto,
            tac::Instr::Goto {
                label: f_branch.next(),
            },
        );

        return output;
    }

    fn in_type(&self) -> DataType {
        return DataType::Bool(false);
    }

    fn out_type(&self) -> DataType {
        return DataType::Bool(false);
    }
}

pub trait Stmt {
    fn emit(self: Box<Self>, prog: &mut tac::Prog);
}

pub struct If {
    pub expr: Box<dyn Expr>,
    pub stmt: Box<dyn Stmt>,
}

impl Stmt for If {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Resolve the expr
        let b = self.expr.emit(prog);
        let if1 = prog.add_temp_instr();

        // Execute the statement if true
        self.stmt.emit(prog);

        // Point if to correct labels
        prog.mod_instr(
            if1,
            tac::Instr::IfExpr {
                test: b,
                if_true: tac::Label::CONTINUE,
                if_false: prog.next_label(),
            },
        )
    }
}

pub struct IfElse {
    pub expr: Box<dyn Expr>,
    pub stmt_t: Box<dyn Stmt>,
    pub stmt_f: Box<dyn Stmt>,
}

impl Stmt for IfElse {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Resolve the expr
        let cond = self.expr.emit(prog);
        let if1 = prog.add_temp_instr();

        // Execute the statement if true
        self.stmt_t.emit(prog);

        let goto = prog.add_temp_instr();

        self.stmt_f.emit(prog);

        // Point if/goto to correct labels
        prog.mod_instr(
            if1,
            tac::Instr::IfExpr {
                test: cond,
                if_true: tac::Label::CONTINUE,
                if_false: goto.next(),
            },
        );
        prog.mod_instr(
            goto,
            tac::Instr::Goto {
                label: prog.next_label(),
            },
        )
    }
}

pub struct While {
    pub expr: Box<dyn Expr>,
    pub stmt: Box<dyn Stmt>,
}

impl Stmt for While {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Resolve the expr, then run the stmt & re-eval if true
        let expr_label = prog.next_label();
        let cond = self.expr.emit(prog);
        let if1 = prog.add_temp_instr();
        self.stmt.emit(prog);
        let goto = prog.add_temp_instr();

        // Modify labels of instrs
        prog.mod_instr(
            if1,
            tac::Instr::IfExpr {
                test: cond,
                if_true: tac::Label::CONTINUE,
                if_false: prog.next_label(),
            },
        );
        prog.mod_instr(goto, tac::Instr::Goto { label: expr_label });
    }
}

pub struct Assign {
    pub expr: Box<dyn Expr>,
    pub id: Ident,
}

impl Stmt for Assign {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Resolve the expression
        let t = self.expr.emit(prog);

        // Set the id to the result of the expr
        prog.add_instr(tac::Instr::AssignExpr {
            op: lexer::Token::C('='),
            to: self.id.addr,
            x: t,
            y: tac::Addr(0),
        });
    }
}

pub struct Seq {
    pub stmt1: Box<dyn Stmt>,
    pub stmt2: Box<dyn Stmt>,
}

impl Stmt for Seq {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        self.stmt1.emit(prog);
        self.stmt2.emit(prog);
    }
}

pub struct NullStmt {}

impl Stmt for NullStmt {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {}
}
