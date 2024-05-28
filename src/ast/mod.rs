use crate::{
    lexer,
    stac::{self, DataType, DataVal},
};

pub mod compound;
pub mod func;

pub trait Expr {
    // Resolve the expression, potentially adding instructions to the program,
    // returning the address where the value is stored
    fn emit(self: Box<Self>, prog: &mut stac::Prog);
    // fn in_type(&self, prog: &tac::Prog) -> Vec<DataType>;
    fn out_type(&self, prog: &stac::Prog) -> DataType;
}

#[derive(Clone)]
pub struct Ident {
    pub addr: stac::Addr,
    pub name: lexer::Token,
    pub data_type: DataType,
}

impl Expr for Ident {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        prog.add_instr(stac::Instr::LoadIdent { i: self.addr });
    }

    fn out_type(&self, _prog: &stac::Prog) -> DataType {
        return self.data_type.clone();
    }
}

pub struct Arith {
    pub op: lexer::Token,
    pub x: Box<dyn Expr>,
    pub y: Box<dyn Expr>,
}

impl Expr for Arith {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        let x_type = self.x.out_type(prog);

        self.y.emit(prog);
        self.x.emit(prog);

        match x_type {
            DataType::String => {
                prog.add_instr(stac::Instr::Concat);
            }
            _ => {
                prog.add_instr(stac::Instr::BinaryExpr { op: self.op });
            }
        }
    }

    fn out_type(&self, prog: &stac::Prog) -> DataType {
        use lexer::Token;
        match self.op {
            Token::Eq | Token::Ne | Token::Le | Token::Ge | Token::C('>') | Token::C('<') => {
                return DataType::Bool
            }
            _ => return self.x.out_type(prog),
        }
    }
}

pub struct Unary {
    pub op: lexer::Token,
    pub x: Box<dyn Expr>,
}

impl Expr for Unary {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        self.x.emit(prog);
        prog.add_instr(stac::Instr::UnaryExpr { op: self.op });
    }

    fn out_type(&self, prog: &stac::Prog) -> DataType {
        return self.x.out_type(prog);
    }
}

pub struct Const {
    pub value: DataVal,
    pub data_type: DataType,
}

impl Expr for Const {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        prog.add_instr(stac::Instr::LoadConst { v: self.value });
    }

    fn out_type(&self, _prog: &stac::Prog) -> DataType {
        return self.data_type.clone();
    }
}

pub struct BoolOr {
    pub x: Box<dyn Expr>,
    pub y: Box<dyn Expr>,
}

impl Expr for BoolOr {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        self.x.emit(prog);

        // Lazy evaluate the second operand
        let if1 = prog.add_temp_instr();

        self.y.emit(prog);

        let if2 = prog.add_temp_instr();

        // Create true and false branches
        prog.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(false),
        });

        let goto = prog.add_temp_instr();

        let t_branch = prog.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(true),
        });

        // Change the labels of the if/goto statements
        prog.mod_instr(
            if1,
            stac::Instr::IfExpr {
                if_true: t_branch,
                if_false: stac::Label::CONTINUE,
            },
        );
        prog.mod_instr(
            if2,
            stac::Instr::IfExpr {
                if_true: t_branch,
                if_false: stac::Label::CONTINUE,
            },
        );
        prog.mod_instr(
            goto,
            stac::Instr::Goto {
                label: t_branch.next(),
            },
        );
    }

    fn out_type(&self, _prog: &stac::Prog) -> DataType {
        return DataType::Bool;
    }
}

pub struct BoolAnd {
    pub x: Box<dyn Expr>,
    pub y: Box<dyn Expr>,
}

impl Expr for BoolAnd {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        self.x.emit(prog);

        // Lazy evaluate the second operand
        let if1 = prog.add_temp_instr();

        self.y.emit(prog);

        let if2 = prog.add_temp_instr();

        // Create true and false branches
        prog.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(true),
        });

        let goto = prog.add_temp_instr();

        let f_branch = prog.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(false),
        });

        // Change the labels of if/goto statements
        prog.mod_instr(
            if1,
            stac::Instr::IfExpr {
                if_true: stac::Label::CONTINUE,
                if_false: f_branch,
            },
        );
        prog.mod_instr(
            if2,
            stac::Instr::IfExpr {
                if_true: stac::Label::CONTINUE,
                if_false: f_branch,
            },
        );
        prog.mod_instr(
            goto,
            stac::Instr::Goto {
                label: f_branch.next(),
            },
        );
    }

    fn out_type(&self, _prog: &stac::Prog) -> DataType {
        return DataType::Bool;
    }
}

pub struct BoolNot {
    pub x: Box<dyn Expr>,
}

impl Expr for BoolNot {
    // Implement not using jumps instead of a dedicated op in AssignExpr, i guess...
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        self.x.emit(prog);

        // Lazy evaluate the second operand
        let if1 = prog.add_temp_instr();

        // Create true and false branches
        prog.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(false),
        });

        let goto = prog.add_temp_instr();

        let f_branch = prog.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(true), // false branch returns true
        });

        // Change the labels of if/goto statements
        prog.mod_instr(
            if1,
            stac::Instr::IfExpr {
                if_true: stac::Label::CONTINUE,
                if_false: f_branch,
            },
        );
        prog.mod_instr(
            goto,
            stac::Instr::Goto {
                label: f_branch.next(),
            },
        );
    }

    fn out_type(&self, _prog: &stac::Prog) -> DataType {
        return DataType::Bool;
    }
}

pub trait Stmt {
    fn emit(self: Box<Self>, prog: &mut stac::Prog);
}

pub struct If {
    pub expr: Box<dyn Expr>,
    pub stmt: Box<dyn Stmt>,
}

impl Stmt for If {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        // Resolve the expr
        self.expr.emit(prog);
        let if1 = prog.add_temp_instr();

        // Execute the statement if true
        self.stmt.emit(prog);

        // Point if to correct labels
        prog.mod_instr(
            if1,
            stac::Instr::IfExpr {
                if_true: stac::Label::CONTINUE,
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
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        // Resolve the expr
        self.expr.emit(prog);
        let if1 = prog.add_temp_instr();

        // Execute the statement if true
        self.stmt_t.emit(prog);

        let goto = prog.add_temp_instr();

        self.stmt_f.emit(prog);

        // Point if/goto to correct labels
        prog.mod_instr(
            if1,
            stac::Instr::IfExpr {
                if_true: stac::Label::CONTINUE,
                if_false: goto.next(),
            },
        );
        prog.mod_instr(
            goto,
            stac::Instr::Goto {
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
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        // Resolve the expr, then run the stmt & re-eval if true
        let expr_label = prog.next_label();
        self.expr.emit(prog);
        let if1 = prog.add_temp_instr();
        self.stmt.emit(prog);
        let goto = prog.add_temp_instr();

        // Modify labels of instrs
        prog.mod_instr(
            if1,
            stac::Instr::IfExpr {
                if_true: stac::Label::CONTINUE,
                if_false: prog.next_label(),
            },
        );
        prog.mod_instr(goto, stac::Instr::Goto { label: expr_label });
    }
}

pub struct Assign {
    pub expr: Box<dyn Expr>,
    pub id: Ident,
}

impl Stmt for Assign {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        // Resolve the expression
        self.expr.emit(prog);

        // Set the id to the result of the expr
        prog.add_instr(stac::Instr::StoreIdent { i: self.id.addr });
    }
}

pub struct Seq {
    pub stmt1: Box<dyn Stmt>,
    pub stmt2: Box<dyn Stmt>,
}

impl Stmt for Seq {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        self.stmt1.emit(prog);
        self.stmt2.emit(prog);
    }
}

pub struct NullStmt {}

impl Stmt for NullStmt {
    fn emit(self: Box<Self>, _prog: &mut stac::Prog) {}
}
