use crate::{ast::Const, ast::Expr, ast::Ident, ast::Stmt, tac, tac::DataType, tac::DataVal};

pub struct ArrayLiteral {
    pub values: Vec<Box<dyn Expr>>,
}

impl Expr for ArrayLiteral {
    fn emit(mut self: Box<Self>, prog: &mut tac::Prog) {
        // Create the array
        prog.add_instr(tac::Instr::LoadConst {
            v: DataVal::Integer(self.values.len() as i64),
        });
        prog.add_instr(tac::Instr::ArrayCreate);

        // For each value in the array, evaluate it, and set it in the array
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

            prog.add_instr(tac::Instr::LoadConst {
                v: DataVal::Integer(idx as i64),
            });

            v.emit(prog);

            prog.add_instr(tac::Instr::ArraySet);
        }
    }

    fn in_type(&self) -> Vec<DataType> {
        return vec![self.values[0].out_type(); self.values.len()];
    }

    fn out_type(&self) -> DataType {
        return DataType::Array(Box::new(self.values[0].out_type()));
    }
}

pub struct ArrayIndex {
    pub arr: Box<dyn Expr>,
    pub index: Box<dyn Expr>,
}

impl Expr for ArrayIndex {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        self.arr.emit(prog);
        self.index.emit(prog);
        prog.add_instr(tac::Instr::ArrayGet);
    }

    fn in_type(&self) -> Vec<DataType> {
        return vec![self.arr.out_type(), DataType::Integer];
    }

    fn out_type(&self) -> DataType {
        return *self.arr.out_type().into_array().unwrap();
    }
}

pub struct AssignArray {
    pub expr: Box<dyn Expr>,
    pub id: Ident,
    pub index: Box<dyn Expr>,
}

impl Stmt for AssignArray {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Load the array
        prog.add_instr(tac::Instr::LoadIdent { i: self.id.addr });

        // Resolve the index
        self.index.emit(prog);

        // Resolve the expression
        self.expr.emit(prog);

        // Set the value in the array
        prog.add_instr(tac::Instr::ArraySet);

        // Set the id to the array
        prog.add_instr(tac::Instr::StoreIdent { i: self.id.addr });
    }
}
