// A func call can be used as an expression when it only returns one variable
// pub struct FuncCall {
//     pub params: Vec<Box<dyn Expr>>,
//     pub func: Box<dyn Expr>,
// }

// impl Expr for FuncCall {
//     fn emit(mut self: Box<Self>, prog: &mut tac::Prog) {
//         let func_addr = self.func.emit(prog);

//         // Evaluate all of the parameters
//         for (i, p_ref) in self.params.iter_mut().enumerate() {
//             let p = std::mem::replace(
//                 p_ref,
//                 Box::new(Const {
//                     value: DataVal::Bool(false),
//                     data_type: DataType::Bool,
//                 }),
//             );
//             let from = p.emit(prog);

//             // Find where the paramater should be stored
//             let param_addr = prog.allocate_var();
//             let index = prog.allocate_var();
//             prog.add_instr(tac::Instr::LoadConst {
//                 v: tac::DataVal::Integer(i as i64),
//                 addr: index,
//             });
//             prog.add_instr(tac::Instr::ArrayGet {
//                 index: index,
//                 arr: params_mem,
//                 to: param_addr,
//             });

//             // Assign the value to the parameter
//             prog.add_instr(tac::Instr::AssignExpr {
//                 op: lexer::Token::C('='),
//                 to: param_addr,
//                 x: from,
//                 y: tac::Addr(0),
//             });
//         }

//         // Call the function
//         prog.add_instr(tac::Instr::Call { label });

//         // Return the first return
//         return returns_mem[0];
//     }

//     fn in_type(&self) -> Vec<DataType> {
//         // Since func call isn't an infix operator, it doesn't have an in type
//         return self.params.iter().map(|p| p.out_type()).collect();
//     }

//     fn out_type(&self) -> DataType {
//         match self.func.out_type() {
//             DataType::Func { params, returns } => {
//                 if returns.len() == 1 {
//                     return returns[0];
//                 } else {
//                     panic!("cannot create expression for function that has multiple returns")
//                 }
//             }
//             _ => panic!("cannot call non-function"),
//         }
//     }
// }

// impl Stmt for FuncCall {
//     // fn emit(mut self: Box<Self>, prog: &mut tac::Prog) {
//     //     // Evaluate all of the parameters
//     //     for (i, p_ref) in self.params.iter_mut().enumerate() {
//     //         let p = std::mem::replace(
//     //             p_ref,
//     //             Box::new(Const {
//     //                 value: DataVal::Bool(false),
//     //             }),
//     //         );
//     //         let from = p.emit(prog);
//     //         prog.add_instr(tac::Instr::AssignExpr {
//     //             op: lexer::Token::C('='),
//     //             to: params_mem[i],
//     //             x: from,
//     //             y: tac::Addr(0),
//     //         });
//     //     }

//     //     // Call the function
//     //     prog.add_instr(tac::Instr::Call { label });
//     // }
// }

// pub struct FuncImpl {
//     pub name: lexer::Token,
//     pub name_addr: tac::Addr,
//     pub body: Box<dyn Stmt>,

//     pub params: Vec<DataType>,
//     pub returns: Vec<DataType>,
//     pub params_mem: Vec<tac::Addr>,
//     pub returns_mem: Vec<tac::Addr>,
// }

// impl Stmt for FuncImpl {
//     fn emit(self: Box<Self>, prog: &mut tac::Prog) {
//         // Goto after the function definition
//         let goto = prog.add_temp_instr();

//         // Assign this function to the variable where it is stored
//         prog.add_instr(tac::Instr::LoadConst {
//             addr: self.name_addr,
//             v: tac::DataVal::Func {
//                 params: self.params,
//                 returns: self.returns,
//                 label: prog.next_label().next(),
//                 params_mem: self.params_mem,
//                 returns_mem: self.returns_mem,
//             },
//         });

//         // Emit the body
//         self.body.emit(prog);
//         prog.add_instr(tac::Instr::Return {});

//         prog.mod_instr(
//             goto,
//             tac::Instr::Goto {
//                 label: prog.next_label(),
//             },
//         )
//     }
// }

// pub struct MultiAssign {
//     pub call: Label,
//     pub
// }
