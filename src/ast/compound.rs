use crate::{ast::Const, ast::Expr, ast::Ident, ast::Stmt, stac, stac::DataType, stac::DataVal};

pub struct ArrayLiteral {
    pub values: Vec<Box<dyn Expr>>,
}

impl Expr for ArrayLiteral {
    fn emit(mut self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Create the array
        block.add_instr(stac::Instr::LoadConst {
            v: DataVal::Integer(self.values.len() as i64),
        });
        block.add_instr(stac::Instr::CompoundCreate);

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

            block.add_instr(stac::Instr::LoadConst {
                v: DataVal::Integer(idx as i64),
            });

            v.emit(prog, block);

            block.add_instr(stac::Instr::CompoundSet);
        }
    }

    fn out_type(&self, prog: &stac::Prog) -> DataType {
        return DataType::Array(Box::new(self.values[0].out_type(prog)));
    }
}

pub struct ArrayIndex {
    pub arr: Box<dyn Expr>,
    pub index: Box<dyn Expr>,
}

impl Expr for ArrayIndex {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        self.arr.emit(prog, block);
        self.index.emit(prog, block);
        block.add_instr(stac::Instr::CompoundGet);
    }

    fn out_type(&self, prog: &stac::Prog) -> DataType {
        return *self.arr.out_type(prog).into_array().unwrap();
    }
}

pub struct AssignArray {
    pub expr: Box<dyn Expr>,
    pub id: Ident,
    pub index: Box<dyn Expr>,
}

impl Stmt for AssignArray {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Load the array
        block.add_instr(stac::Instr::LoadIdent { i: self.id.addr });

        // Resolve the index
        self.index.emit(prog, block);

        // Resolve the expression
        self.expr.emit(prog, block);

        // Set the value in the array
        block.add_instr(stac::Instr::CompoundSet);

        // Set the id to the array
        block.add_instr(stac::Instr::StoreIdent { i: self.id.addr });
    }
}

pub struct StructAccess {
    pub expr: Box<dyn Expr>,
    pub field: String,
}

impl Expr for StructAccess {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Lookup index for field
        let strct = prog
            .user_structs
            .get(&self.expr.out_type(prog).into_struct().unwrap())
            .unwrap()
            .to_owned();

        self.expr.emit(prog, block);

        let idx = *strct.names.get(&self.field).unwrap();
        block.add_instr(stac::Instr::LoadConst {
            v: DataVal::Integer(idx as i64),
        });

        block.add_instr(stac::Instr::CompoundGet);
    }

    fn out_type(&self, prog: &stac::Prog) -> DataType {
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
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Resolve the struct
        let strct = prog.user_structs.get(&self.strct).unwrap().to_owned();

        // Create an empty struct
        block.add_instr(stac::Instr::LoadConst {
            v: DataVal::Integer(strct.types.len() as i64),
        });
        block.add_instr(stac::Instr::CompoundCreate);

        // Evaluate each value and assign it to the field
        let mut remaining_fields = strct.names.clone();
        for (field, ref mut value) in self.values {
            let idx = *strct.names.get(&field).unwrap();
            block.add_instr(stac::Instr::LoadConst {
                v: DataVal::Integer(idx as i64),
            });

            let val = std::mem::replace(
                value,
                Box::new(Const {
                    value: DataVal::Bool(false),
                    data_type: DataType::Bool,
                }),
            );
            val.emit(prog, block);

            block.add_instr(stac::Instr::CompoundSet);
            remaining_fields.remove(&field);
        }

        // Store the default value in the struct for any remaining fields
        for (_, idx) in remaining_fields {
            block.add_instr(stac::Instr::LoadConst {
                v: DataVal::Integer(idx as i64),
            });

            // Get the default value for the type
            let val = DataVal::default_for(strct.types[idx].clone(), &prog.user_structs);
            block.add_instr(stac::Instr::LoadConst { v: val });
            block.add_instr(stac::Instr::CompoundSet);
        }
    }

    fn out_type(&self, _prog: &stac::Prog) -> DataType {
        return DataType::Struct(self.strct.clone());
    }
}

pub struct AssignStruct {
    pub id: Ident,
    pub field: String,
    pub expr: Box<dyn Expr>,
}

impl Stmt for AssignStruct {
    fn emit(self: Box<Self>, prog: &mut stac::Prog, block: &mut stac::Block) {
        // Load the struct
        block.add_instr(stac::Instr::LoadIdent { i: self.id.addr });

        // Resolve the field to an index
        let strct = prog
            .user_structs
            .get(&self.id.data_type.into_struct().unwrap())
            .unwrap();

        let idx = *strct.names.get(&self.field).unwrap();
        block.add_instr(stac::Instr::LoadConst {
            v: DataVal::Integer(idx as i64),
        });

        // Resolve the expression
        self.expr.emit(prog, block);

        // Set the field in the struct
        block.add_instr(stac::Instr::CompoundSet);
    }
}
