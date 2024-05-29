use crate::{ast::Const, ast::Expr, ast::Ident, ast::Stmt, stac, stac::DataType, stac::DataVal};

// A func call can be used as an expression when it only returns one variable
pub struct FuncCall {
    pub params: Vec<Box<dyn Expr>>,
    pub func: Box<dyn Expr>,
}

impl Expr for FuncCall {
    fn emit(mut self: Box<Self>, prog: &mut stac::Prog) {
        // Evaluate all of the parameters
        for idx in 0..self.params.len() {
            let p = std::mem::replace(
                &mut self.params[idx],
                Box::new(Const {
                    value: DataVal::Bool(false),
                    data_type: DataType::Bool,
                }),
            );

            p.emit(prog);
        }

        // Call the function
        self.func.emit(prog);
        prog.add_instr(stac::Instr::Call);
    }

    fn out_type(&self, prog: &stac::Prog) -> DataType {
        let returns = self.func.out_type(prog).into_function().unwrap().1;
        if returns.len() == 1 {
            return returns[0].clone();
        } else {
            panic!("can only use func as expression when it has one return")
        }
    }
}

impl Stmt for FuncCall {
    fn emit(mut self: Box<Self>, prog: &mut stac::Prog) {
        // Evaluate all of the parameters
        for idx in 0..self.params.len() {
            let p = std::mem::replace(
                &mut self.params[idx],
                Box::new(Const {
                    value: DataVal::Bool(false),
                    data_type: DataType::Bool,
                }),
            );

            p.emit(prog);
        }

        // Call the function
        let returns_count = self.func.out_type(prog).into_function().unwrap().1.len();
        self.func.emit(prog);
        prog.add_instr(stac::Instr::Call);

        // Discard the returns
        for _ in 0..returns_count {
            prog.add_instr(stac::Instr::Discard);
        }
    }
}

pub struct FuncImpl {
    pub id: Ident,
    pub body: Box<dyn Stmt>,

    pub params: Vec<Ident>,
    pub returns: Vec<DataType>,
}

impl Stmt for FuncImpl {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        // Assign this function to the variable where it is stored
        prog.add_instr(stac::Instr::LoadConst {
            v: stac::DataVal::Function(prog.next_label().next().next().next()),
            // load, store, goto
            // next, next, next
        });
        prog.add_instr(stac::Instr::StoreIdent { i: self.id.addr });

        // Goto after the function definition
        let goto = prog.add_temp_instr();

        // Load the parameters into their assigned variables
        for param in self.params.iter().rev() {
            prog.add_instr(stac::Instr::StoreIdent { i: param.addr });
        }

        // Emit the body
        self.body.emit(prog);

        // just in case the function doesn't have a final return
        prog.add_instr(stac::Instr::Return {});

        prog.mod_instr(
            goto,
            stac::Instr::Goto {
                label: prog.next_label(),
            },
        )
    }
}

pub struct ExternFuncImpl {
    pub id: Ident,
    pub params_count: usize,
}

impl Stmt for ExternFuncImpl {
    fn emit(self: Box<Self>, prog: &mut stac::Prog) {
        // Assign this function to the variable where it is stored
        prog.add_instr(stac::Instr::LoadConst {
            v: stac::DataVal::Function(prog.next_label().next().next().next()),
            // load, store, goto
            // next, next, next
        });
        prog.add_instr(stac::Instr::StoreIdent { i: self.id.addr });

        // Goto after the function definition
        let goto = prog.add_temp_instr();

        // Add the name of this function to the eval stack
        prog.add_instr(stac::Instr::LoadConst {
            v: DataVal::String(self.id.name.into_word().unwrap()),
        });

        // Make the extern call
        prog.add_instr(stac::Instr::ExternCall {
            params_count: self.params_count,
        });

        // Return
        prog.add_instr(stac::Instr::Return {});

        prog.mod_instr(
            goto,
            stac::Instr::Goto {
                label: prog.next_label(),
            },
        )
    }
}

pub struct Return {
    pub values: Vec<Box<dyn Expr>>,
}

impl Stmt for Return {
    fn emit(mut self: Box<Self>, prog: &mut stac::Prog) {
        // Evaluate each item, leaving it on the stack
        for idx in 0..self.values.len() {
            // In order to emit it, we need to own the value, which means
            // we need to replace the value in the array with somthing
            let v = std::mem::replace(
                &mut self.values[idx],
                Box::new(Const {
                    value: DataVal::Bool(false),
                    data_type: DataType::Bool,
                }),
            );

            v.emit(prog);
        }

        prog.add_instr(stac::Instr::Return);
    }
}
