use crate::{
    lexer,
    stac::{self, DataType, DataVal},
};

pub mod compound;
pub mod func;

pub trait Expr {
    // Resolve the expression, potentially adding instructions to the program,
    // returning the address where the value is stored
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block);
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
    fn emit(self: Box<Self>, _prog: &mut stac::Prog, block: &mut stac::Block) {
        block.add_instr(stac::Instr::LoadIdent { i: self.addr });
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
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        let x_type = self.x.out_type(prog);

        self.y.emit(prog, block);
        self.x.emit(prog, block);

        match x_type {
            DataType::String => {
                block.add_instr(stac::Instr::Concat);
            }
            _ => {
                block.add_instr(stac::Instr::BinaryExpr { op: self.op });
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
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        self.x.emit(prog, block);
        block.add_instr(stac::Instr::UnaryExpr { op: self.op });
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
    fn emit(self: Box<Self>, _prog: &mut stac::Prog, block: &mut stac::Block) {
        block.add_instr(stac::Instr::LoadConst { v: self.value });
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
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        self.x.emit(prog, block);

        // Lazy evaluate the second operand
        let mut true_block = stac::Block::new();
        let mut initially_false_block = stac::Block::new();
        let mut finally_false_block = stac::Block::new();

        true_block.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(true),
        });

        finally_false_block.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(false),
        });

        let true_label = prog.add_block(true_block);
        let finally_false_label = prog.add_block(finally_false_block);

        self.y.emit(prog, &mut initially_false_block);

        initially_false_block.add_instr(stac::Instr::IfExpr {
            if_true: true_label,
            if_false: finally_false_label,
        });

        let initially_false_label = prog.add_block(initially_false_block);
        block.add_instr(stac::Instr::IfExpr {
            if_true: true_label,
            if_false: initially_false_label,
        })
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
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        self.x.emit(prog, block);

        // Lazy evaluate the second operand
        let mut false_block = stac::Block::new();
        let mut initially_true_block = stac::Block::new();
        let mut finally_true_block = stac::Block::new();

        false_block.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(false),
        });

        finally_true_block.add_instr(stac::Instr::LoadConst {
            v: DataVal::Bool(true),
        });

        let false_label = prog.add_block(false_block);
        let finally_true_label = prog.add_block(finally_true_block);

        self.y.emit(prog, &mut initially_true_block);

        initially_true_block.add_instr(stac::Instr::IfExpr {
            if_true: finally_true_label,
            if_false: false_label,
        });

        let initially_true_label = prog.add_block(initially_true_block);
        block.add_instr(stac::Instr::IfExpr {
            if_true: initially_true_label,
            if_false: false_label,
        })
    }

    fn out_type(&self, _prog: &stac::Prog) -> DataType {
        return DataType::Bool;
    }
}

pub struct BoolNot {
    pub x: Box<dyn Expr>,
}

impl Expr for BoolNot {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        self.x.emit(prog, block);
        block.add_instr(stac::Instr::UnaryExpr {
            op: lexer::Token::C('!'),
        });
    }

    fn out_type(&self, _prog: &stac::Prog) -> DataType {
        return DataType::Bool;
    }
}

pub trait Stmt {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block);
}

pub struct If {
    pub expr: Box<dyn Expr>,
    pub stmt: Box<dyn Stmt>,
}

impl Stmt for If {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Resolve the expr
        self.expr.emit(prog, block);

        // Execute the statement if true
        let mut true_block = stac::Block::new();
        self.stmt.emit(prog, &mut true_block);
        let true_label = prog.add_block(true_block);

        // Point if to correct labels
        block.add_instr(stac::Instr::IfExpr {
            if_true: true_label,
            if_false: stac::Label::CONTINUE,
        })
    }
}

pub struct IfElse {
    pub expr: Box<dyn Expr>,
    pub stmt_t: Box<dyn Stmt>,
    pub stmt_f: Box<dyn Stmt>,
}

impl Stmt for IfElse {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Resolve the expr
        self.expr.emit(prog, block);

        // Create the true block
        let mut true_block = stac::Block::new();
        self.stmt_t.emit(prog, &mut true_block);
        let true_label = prog.add_block(true_block);

        // Create the false block
        let mut false_block = stac::Block::new();
        self.stmt_f.emit(prog, &mut false_block);
        let false_label = prog.add_block(false_block);

        // Point if to correct labels
        block.add_instr(stac::Instr::IfExpr {
            if_true: true_label,
            if_false: false_label,
        })
    }
}

pub struct While {
    pub expr: Box<dyn Expr>,
    pub stmt: Box<dyn Stmt>,
}

impl Stmt for While {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Resolve the expr, then run the stmt & re-eval if true
        let stmt_label = prog.add_temp_block();

        let mut expr_block = stac::Block::new();
        self.expr.emit(prog, &mut expr_block);
        expr_block.add_instr(stac::Instr::IfExpr {
            if_true: stmt_label,
            if_false: stac::Label::CONTINUE, // will automatically unwind the entire call stack
        });
        let expr_label = prog.add_block(expr_block);

        let mut stmt_block = stac::Block::new();
        self.stmt.emit(prog, &mut stmt_block);
        stmt_block.add_instr(stac::Instr::Goto { label: expr_label });
        prog.mod_block(stmt_block, stmt_label);

        block.add_instr(stac::Instr::Goto { label: expr_label });
    }
}

pub struct Assign {
    pub expr: Box<dyn Expr>,
    pub id: Ident,
}

impl Stmt for Assign {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Resolve the expression
        self.expr.emit(prog, block);

        // Set the id to the result of the expr
        block.add_instr(stac::Instr::StoreIdent { i: self.id.addr });
    }
}

pub struct Seq {
    pub stmt1: Box<dyn Stmt>,
    pub stmt2: Box<dyn Stmt>,
}

impl Stmt for Seq {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        self.stmt1.emit(prog, block);
        self.stmt2.emit(prog, block);
    }
}

pub struct NullStmt {}

impl Stmt for NullStmt {
    fn emit(self: Box<Self>, _prog: &mut stac::Prog, _block: &mut stac::Block) {}
}
