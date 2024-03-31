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
        prog.add_instr(tac::Instr::CompoundCreate);

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

            prog.add_instr(tac::Instr::CompoundSet);
        }
    }

    fn in_type(&self, prog: &tac::Prog) -> Vec<DataType> {
        return vec![self.values[0].out_type(prog); self.values.len()];
    }

    fn out_type(&self, prog: &tac::Prog) -> DataType {
        return DataType::Array(Box::new(self.values[0].out_type(prog)));
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
        prog.add_instr(tac::Instr::CompoundGet);
    }

    fn in_type(&self, prog: &tac::Prog) -> Vec<DataType> {
        return vec![self.arr.out_type(prog), DataType::Integer];
    }

    fn out_type(&self, prog: &tac::Prog) -> DataType {
        return *self.arr.out_type(prog).into_array().unwrap();
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
        prog.add_instr(tac::Instr::CompoundSet);

        // Set the id to the array
        prog.add_instr(tac::Instr::StoreIdent { i: self.id.addr });
    }
}

pub struct StructAccess {
    pub expr: Box<dyn Expr>,
    pub field: String,
}

impl Expr for StructAccess {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Lookup index for field
        let strct = prog
            .user_structs
            .get(&self.expr.out_type(prog).into_struct().unwrap())
            .unwrap()
            .to_owned();

        self.expr.emit(prog);

        let idx = *strct.names.get(&self.field).unwrap();
        prog.add_instr(tac::Instr::LoadConst {
            v: DataVal::Integer(idx as i64),
        });

        prog.add_instr(tac::Instr::CompoundGet);
    }

    fn in_type(&self, prog: &tac::Prog) -> Vec<DataType> {
        return vec![self.out_type(prog)];
    }

    fn out_type(&self, prog: &tac::Prog) -> DataType {
        let name = self.expr.out_type(prog).into_struct().unwrap();
        let strct = prog.user_structs.get(&name).unwrap().to_owned();
        return strct.types[*strct.names.get(&self.field).unwrap()].clone();
    }
}

pub struct StructLiteral {
    pub strct: String,
    pub values: Vec<(String, Box<dyn Expr>)>,
}

impl Expr for StructLiteral {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Resolve the struct
        let strct = prog.user_structs.get(&self.strct).unwrap().to_owned();

        // Create an empty struct
        prog.add_instr(tac::Instr::LoadConst {
            v: DataVal::Integer(strct.types.len() as i64),
        });
        prog.add_instr(tac::Instr::CompoundCreate);

        // Evaluate each value and assign it to the field
        for (field, ref mut value) in self.values {
            let idx = *strct.names.get(&field).unwrap();
            prog.add_instr(tac::Instr::LoadConst {
                v: DataVal::Integer(idx as i64),
            });

            let val = std::mem::replace(
                value,
                Box::new(Const {
                    value: DataVal::Bool(false),
                    data_type: DataType::Bool,
                }),
            );
            val.emit(prog);

            prog.add_instr(tac::Instr::CompoundSet);
        }
    }

    fn in_type(&self, _prog: &tac::Prog) -> Vec<DataType> {
        // TODO Fix
        return vec![];
    }

    fn out_type(&self, _prog: &tac::Prog) -> DataType {
        return DataType::Struct(self.strct.clone());
    }
}

pub struct AssignStruct {
    pub id: Ident,
    pub field: String,
    pub expr: Box<dyn Expr>,
}

impl Stmt for AssignStruct {
    fn emit(self: Box<Self>, prog: &mut tac::Prog) {
        // Load the struct
        prog.add_instr(tac::Instr::LoadIdent { i: self.id.addr });

        // Resolve the field to an index
        let strct = prog
            .user_structs
            .get(&self.id.data_type.into_struct().unwrap())
            .unwrap();

        let idx = *strct.names.get(&self.field).unwrap();
        prog.add_instr(tac::Instr::LoadConst {
            v: DataVal::Integer(idx as i64),
        });

        // Resolve the expression
        self.expr.emit(prog);

        // Set the field in the struct
        prog.add_instr(tac::Instr::CompoundSet);
    }
}
