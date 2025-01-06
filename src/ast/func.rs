use crate::{ast::Const, ast::Expr, ast::Ident, ast::Stmt, stac, stac::DataType, stac::DataVal};

// A func call can be used as an expression when it only returns one variable
pub struct FuncCall {
    pub params: Vec<Box<dyn Expr>>,
    pub func: String,
}

impl Expr for FuncCall {
    fn emit(mut self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Evaluate all of the parameters
        for idx in 0..self.params.len() {
            let p = std::mem::replace(
                &mut self.params[idx],
                Box::new(Const {
                    value: DataVal::Bool(false),
                    data_type: DataType::Bool,
                }),
            );

            p.emit(prog, block);
        }

        // Call the function
        block.add_instr(stac::Instr::Call {
            label: prog.user_functions.get(&self.func).unwrap().label,
        });
    }

    fn out_type(&self, prog: &stac::Prog) -> DataType {
        let returns = &prog.user_functions.get(&self.func).unwrap().returns;
        if returns.len() == 1 {
            return returns[0].clone();
        } else {
            panic!("can only use func as expression when it has one return")
        }
    }
}

impl Stmt for FuncCall {
    fn emit(mut self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Evaluate all of the parameters
        for idx in 0..self.params.len() {
            let p = std::mem::replace(
                &mut self.params[idx],
                Box::new(Const {
                    value: DataVal::Bool(false),
                    data_type: DataType::Bool,
                }),
            );

            p.emit(prog, block);
        }

        // Call the function
        let returns_count = prog.user_functions.get(&self.func).unwrap().returns.len();
        block.add_instr(stac::Instr::Call {
            label: prog.user_functions.get(&self.func).unwrap().label,
        });

        // Discard the returns
        for _ in 0..returns_count {
            block.add_instr(stac::Instr::Discard);
        }
    }
}

pub struct FuncImpl {
    pub name: String,
    pub body: Box<dyn Stmt>,

    pub params: Vec<Ident>,
}

impl Stmt for FuncImpl {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, _block: &mut stac::Block) {
        let mut body_block = stac::Block::new();

        // Load the parameters into their assigned variables
        for param in self.params.iter().rev() {
            body_block.add_instr(stac::Instr::StoreIdent { i: param.addr });
        }

        // Emit the body
        self.body.emit(prog, &mut body_block);
        let body_label = prog.add_block(body_block);

        // Add the label of the function to the program
        prog.user_functions
            .entry(self.name)
            .and_modify(|f| f.label = body_label);
    }
}

pub struct ExternFuncImpl {
    pub name: String,
    pub params_count: usize,
}

impl Stmt for ExternFuncImpl {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, _block: &mut stac::Block) {
        let mut body_block = stac::Block::new();

        // Add the name of this function to the eval stack
        body_block.add_instr(stac::Instr::LoadConst {
            v: DataVal::String(self.name.clone()),
        });

        // Make the extern call
        body_block.add_instr(stac::Instr::ExternCall {
            params_count: self.params_count,
        });
        let body_label = prog.add_block(body_block);

        // Add the label of the function to the program
        prog.user_functions
            .entry(self.name)
            .and_modify(|f| f.label = body_label);
    }
}

pub struct Return {
    pub values: Vec<Box<dyn Expr>>,
}

impl Stmt for Return {
    fn emit(mut self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
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

            v.emit(prog, block);
        }

        block.add_instr(stac::Instr::Return);
    }
}
