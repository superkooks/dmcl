use crate::{
    lexer,
    tac::{self, DataType, DataVal, Label},
};

pub trait Expr {
    // Resolve the expression, potentially adding instructions to the program,
    // returning the address where the value is stored
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr;
}

#[derive(Clone)]
pub struct Ident {
    pub name: lexer::Token,
    pub addr: tac::Addr,
    pub data_type: DataType,
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

    // fn in_type(&self) -> Vec<DataType> {
    //     return vec![];
    // }

    // fn out_type(&self) -> DataType {
    //     return self.data_type;
    // }
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

    // fn in_type(&self) -> Vec<DataType> {
    //     let x = self.x.out_type();
    //     let y = self.y.out_type();
    //     if x == y {
    //         return vec![x, y];
    //     } else {
    //         panic!("type error: inputs to arith have different types")
    //     }
    // }

    // fn out_type(&self) -> DataType {
    //     use lexer::Token;
    //     match self.op {
    //         Token::Eq | Token::Ne | Token::Le | Token::Ge | Token::C('>') | Token::C('<') => {
    //             return DataType::Bool
    //         }
    //         _ => return self.x.out_type(),
    //     }
    // }
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

    // fn in_type(&self) -> Vec<DataType> {
    //     return vec![self.x.out_type()];
    // }

    // fn out_type(&self) -> DataType {
    //     return self.in_type()[0];
    // }
}

pub struct Const {
    pub value: DataVal,
}

impl Expr for Const {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let temp = prog.allocate_var();
        prog.add_instr(tac::Instr::StoreConst {
            v: self.value,
            addr: temp,
        });
        return temp;
    }

    //     fn in_type(&self) -> Vec<DataType> {
    //         return vec![];
    //     }

    // fn out_type(&self) -> DataType {
    //     // return self.data_type.clone();
    //     panic!("ignore")
    // }
}

pub struct Array {
    pub values: Vec<Box<dyn Expr>>,
}

impl Expr for Array {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let len = prog.allocate_var();
        prog.add_instr(tac::Instr::StoreConst {
            v: DataVal::Integer(self.values.len() as i64),
            addr: len,
        });

        let temp = prog.allocate_var();
        prog.add_instr(tac::Instr::ArrayCreate {
            arr: temp,
            count: len,
        });

        return temp;
    }

    // fn in_type(&self) -> DataType {
    //     unimplemented!()
    // }
    // fn out_type(&self) -> DataType {
    //     unimplemented!()
    // }
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
            v: DataVal::Bool(false),
            addr: output,
        });

        let goto = prog.add_temp_instr();

        let t_branch = prog.add_instr(tac::Instr::StoreConst {
            v: DataVal::Bool(true),
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

    // fn in_type(&self) -> Vec<DataType> {
    //     return vec![DataType::Bool, DataType::Bool];
    // }

    // fn out_type(&self) -> DataType {
    //     return DataType::Bool;
    // }
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
            v: DataVal::Bool(true),
            addr: output,
        });

        let goto = prog.add_temp_instr();

        let f_branch = prog.add_instr(tac::Instr::StoreConst {
            v: DataVal::Bool(false),
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

    // fn in_type(&self) -> Vec<DataType> {
    //     return vec![DataType::Bool, DataType::Bool];
    // }

    // fn out_type(&self) -> DataType {
    //     return DataType::Bool;
    // }
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
            v: DataVal::Bool(false),
            addr: output,
        });

        let goto = prog.add_temp_instr();

        let f_branch = prog.add_instr(tac::Instr::StoreConst {
            v: DataVal::Bool(true), // false branch returns true
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

    // fn in_type(&self) -> DataType {
    //     return DataType::Bool;
    // }

    // fn out_type(&self) -> DataType {
    //     return DataType::Bool;
    // }
}

// A func call can be used as an expression when it only returns one variable
pub struct FuncCall {
    pub params: Vec<Box<dyn Expr>>,
    pub func: Box<dyn Expr>,
}

impl Expr for FuncCall {
    fn emit(mut self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let func_addr = self.func.emit(prog);

        // Get the array of where paramaters should be stored for the call
        let params_mem = prog.allocate_var();
        let zero = prog.allocate_var();
        prog.add_instr(tac::Instr::StoreConst {
            v: tac::DataVal::Integer(0),
            addr: zero,
        });
        prog.add_instr(tac::Instr::ArrayGet {
            index: zero,
            arr: func_addr,
            to: params_mem,
        });

        // Evaluate all of the parameters
        for (i, p_ref) in self.params.iter_mut().enumerate() {
            let p = std::mem::replace(
                p_ref,
                Box::new(Const {
                    value: DataVal::Bool(false),
                }),
            );
            let from = p.emit(prog);

            // Find where the paramater should be stored
            let param_addr = prog.allocate_var();
            let index = prog.allocate_var();
            prog.add_instr(tac::Instr::StoreConst {
                v: tac::DataVal::Integer(i as i64),
                addr: index,
            });
            prog.add_instr(tac::Instr::ArrayGet {
                index: index,
                arr: params_mem,
                to: param_addr,
            });

            // Assign the value to the parameter
            prog.add_instr(tac::Instr::AssignExpr {
                op: lexer::Token::C('='),
                to: param_addr,
                x: from,
                y: tac::Addr(0),
            });
        }

        // Call the function
        prog.add_instr(tac::Instr::Call { label });

        // Return the first return
        return returns_mem[0];
    }

    // fn in_type(&self) -> Vec<DataType> {
    //     // Since func call isn't an infix operator, it doesn't have an in type
    //     return self.params.iter().map(|p| p.out_type()).collect();
    // }

    // fn out_type(&self) -> DataType {}
}

impl Stmt for FuncCall {
    fn emit(mut self: Box<Self>, prog: &mut tac::Prog) {
        // Evaluate all of the parameters
        for (i, p_ref) in self.params.iter_mut().enumerate() {
            let p = std::mem::replace(
                p_ref,
                Box::new(Const {
                    value: DataVal::Bool(false),
                }),
            );
            let from = p.emit(prog);
            prog.add_instr(tac::Instr::AssignExpr {
                op: lexer::Token::C('='),
                to: params_mem[i],
                x: from,
                y: tac::Addr(0),
            });
        }

        // Call the function
        prog.add_instr(tac::Instr::Call { label });
    }
}

pub struct ArrayIndex {
    pub arr: Box<dyn Expr>,
    pub index: Box<dyn Expr>,
}

impl Expr for ArrayIndex {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) -> tac::Addr {
        let out = prog.allocate_var();
        let arr = self.arr.emit(prog);
        let index = self.index.emit(prog);
        prog.add_instr(tac::Instr::ArrayGet {
            index,
            arr,
            to: out,
        });
        return out;
    }

    // fn in_type(&self) -> DataType {
    //     return DataType::Bool;
    // }
    // fn out_type(&self) -> DataType {
    //     return DataType::Bool;
    // }
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
    pub id: Box<dyn Expr>,
    // id = expr
}

impl Stmt for Assign {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Resolve the expression
        let t = self.expr.emit(prog);

        // Resolve the identifier
        let id = self.id.emit(prog);

        // Set the id to the result of the expr
        prog.add_instr(tac::Instr::AssignExpr {
            op: lexer::Token::C('='),
            to: id,
            x: t,
            y: tac::Addr(0),
        });
    }
}

pub struct AssignArray {
    pub expr: Box<dyn Expr>,
    pub id: Box<dyn Expr>,
    pub index: Box<dyn Expr>,
}

impl Stmt for AssignArray {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Resolve the expression
        let t = self.expr.emit(prog);

        // Resolve the identifier
        let id = self.id.emit(prog);

        // Resolve the index
        let index = self.index.emit(prog);

        // Set the id to the result of the expr
        prog.add_instr(tac::Instr::ArraySet {
            from: t,
            arr: id,
            index,
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
    fn emit(self: Box<Self>, _prog: &mut tac::Prog) {}
}

pub struct FuncImpl {
    pub name: lexer::Token,
    pub name_addr: tac::Addr,
    pub body: Box<dyn Stmt>,

    pub params: Vec<DataType>,
    pub returns: Vec<DataType>,
    pub params_mem: Vec<tac::Addr>,
    pub returns_mem: Vec<tac::Addr>,
}

impl Stmt for FuncImpl {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Goto after the function definition
        let goto = prog.add_temp_instr();

        // Assign this function to the variable where it is stored
        prog.add_instr(tac::Instr::StoreConst {
            addr: self.name_addr,
            v: tac::DataVal::Func {
                params: self.params,
                returns: self.returns,
                label: prog.next_label().next(),
                params_mem: self.params_mem,
                returns_mem: self.returns_mem,
            },
        });

        // Emit the body
        self.body.emit(prog);
        prog.add_instr(tac::Instr::Return {});

        prog.mod_instr(
            goto,
            tac::Instr::Goto {
                label: prog.next_label(),
            },
        )
    }
}

// pub struct MultiAssign {
//     pub call: Label,
//     pub
// }
