use std::iter::Map;

use enum_as_inner::EnumAsInner;

use crate::lexer::{self, Token};
use crate::tac;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Addr(pub usize); // Addr of variable in memory

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label(pub usize); // A label to jump to.

impl Label {
    pub const CONTINUE: Label = Label(usize::MAX); // continue execution. used in if.
    pub fn next(&self) -> Label {
        return Label(self.0 + 1);
    }
}

#[derive(Debug, Clone)]
pub enum Instr {
    BinaryExpr {
        op: lexer::Token,
    },

    UnaryExpr {
        op: lexer::Token,
    },

    LoadConst {
        v: tac::DataVal,
    },

    LoadIdent {
        i: Addr,
    },

    StoreIdent {
        i: Addr,
    },

    IfExpr {
        // Special label CONTINUE indicates continuation of execution
        if_true: Label,
        if_false: Label,
    },

    Goto {
        label: Label,
    },

    ArrayGet,
    ArraySet,
    ArrayCreate,

    Call {
        // Sets the return address on the call stack, then does a goto
        label: Label,
    },

    Return,
}

pub struct Struct {
    types: Vec<DataType>,
    names: Map<String, usize>,
}

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum DataType {
    Integer,
    Float,
    Bool,
    Array(Box<DataType>),
    Struct(String), // the name of struct
    Func {
        params: Vec<DataType>,
        returns: Vec<DataType>,
    },
}

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum DataVal {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Compound(Vec<DataVal>),
}

macro_rules! arith {
    ($self:ident, $op:expr) => {{
        let x = $self.eval_stack.pop().unwrap();
        let y = $self.eval_stack.pop().unwrap();
        match x {
            DataVal::Integer(_) => $self.eval_stack.push(DataVal::Integer($op(
                x.into_integer().unwrap(),
                y.into_integer().unwrap(),
            ))),
            DataVal::Float(_) => $self.eval_stack.push(DataVal::Float($op(
                x.into_float().unwrap(),
                y.into_float().unwrap(),
            ))),
            _ => panic!("cannot use arithmetic on those types"),
        }
    }};
}

macro_rules! rel {
    ($self:ident, $op:expr) => {{
        println!("stack {:?}", $self.eval_stack);
        let x = $self.eval_stack.pop().unwrap();
        let y = $self.eval_stack.pop().unwrap();
        println!("stack 2 {:?}", $self.eval_stack);
        match x {
            DataVal::Integer(_) => $self.eval_stack.push(DataVal::Bool($op(
                &x.into_integer().unwrap(),
                &y.into_integer().unwrap(),
            ))),
            DataVal::Float(_) => $self.eval_stack.push(DataVal::Bool($op(
                &x.into_float().unwrap(),
                &y.into_float().unwrap(),
            ))),
            _ => panic!("cannot compare those types"),
        }
    }};
}

// A three address code program
pub struct Prog {
    pub code: Vec<Instr>,

    pub eval_stack: Vec<DataVal>,
    pub variables: Vec<DataVal>,

    ip: usize, // instruction pointer
    call_stack: Vec<usize>,
}

impl Prog {
    pub fn new() -> Prog {
        Prog {
            code: Vec::new(),
            eval_stack: Vec::new(),
            variables: Vec::new(),
            ip: 0,
            call_stack: Vec::new(),
        }
    }

    pub fn allocate_var(&mut self) -> Addr {
        // Doesn't matter what we set it to, just return the address
        self.variables.push(DataVal::Bool(false));
        return Addr(self.variables.len() - 1);
    }

    pub fn add_instr(&mut self, instr: Instr) -> Label {
        self.code.push(instr);
        return Label(self.code.len() - 1);
    }

    pub fn add_temp_instr(&mut self) -> Label {
        // Add a non-executable instruction and return the address
        self.code.push(Instr::Goto {
            label: Label(usize::MAX),
        });
        return Label(self.code.len() - 1);
    }

    pub fn mod_instr(&mut self, label: Label, instr: Instr) {
        self.code[label.0] = instr;
    }

    pub fn next_label(&self) -> Label {
        return Label(self.code.len());
    }

    pub fn execute(&mut self) {
        while self.ip < self.code.len() {
            match self.code[self.ip].clone() {
                Instr::BinaryExpr { op } => match op {
                    Token::C('+') => arith!(self, std::ops::Add::add),
                    Token::C('-') => arith!(self, std::ops::Sub::sub),
                    Token::C('*') => arith!(self, std::ops::Mul::mul),
                    Token::C('/') => arith!(self, std::ops::Div::div),

                    Token::Eq => rel!(self, std::cmp::PartialEq::eq),
                    Token::Ne => rel!(self, std::cmp::PartialEq::ne),

                    Token::C('<') => rel!(self, std::cmp::PartialOrd::lt),
                    Token::Le => rel!(self, std::cmp::PartialOrd::le),
                    Token::C('>') => rel!(self, std::cmp::PartialOrd::gt),
                    Token::Ge => rel!(self, std::cmp::PartialOrd::ge),
                    _ => panic!("unimplemented operator for binary expression"),
                },
                Instr::UnaryExpr { op } => match op {
                    Token::C('-') => {
                        let top = self.eval_stack.pop().unwrap();
                        match top {
                            DataVal::Integer(i) => {
                                self.eval_stack.push(DataVal::Integer(-i));
                            }
                            DataVal::Float(f) => {
                                self.eval_stack.push(DataVal::Float(-f));
                            }
                            _ => panic!("operator unimplemented for data type"),
                        }
                    }
                    _ => panic!("unimplemented operator for unary expression"),
                },
                Instr::LoadConst { v } => self.eval_stack.push(v),
                Instr::LoadIdent { i } => self.eval_stack.push(self.variables[i.0].clone()),
                Instr::StoreIdent { i } => self.variables[i.0] = self.eval_stack.pop().unwrap(),
                Instr::IfExpr { if_true, if_false } => match self.eval_stack.pop().unwrap() {
                    DataVal::Bool(b) => {
                        if b {
                            if if_true != Label::CONTINUE {
                                self.ip = if_true.0;
                            }
                        } else {
                            if if_false != Label::CONTINUE {
                                self.ip = if_false.0;
                            }
                        }
                    }
                    _ => panic!("can only if on bool"),
                },
                Instr::ArrayGet => {
                    let index = self.eval_stack.pop().unwrap().into_integer().unwrap();
                    let arr = self.eval_stack.pop().unwrap().into_compound().unwrap();
                    let val = arr[index as usize].clone();
                    self.eval_stack.push(val);
                }
                Instr::ArraySet => {
                    let val = self.eval_stack.pop().unwrap();
                    let index = self.eval_stack.pop().unwrap().into_integer().unwrap();
                    let mut arr = self.eval_stack.pop().unwrap().into_compound().unwrap();

                    arr[index as usize] = val;
                    self.eval_stack.push(DataVal::Compound(arr));
                }
                Instr::ArrayCreate => {
                    let len = self.eval_stack.pop().unwrap().into_integer().unwrap();
                    let arr = vec![DataVal::Bool(false); len as usize];
                    self.eval_stack.push(DataVal::Compound(arr));
                }
                Instr::Goto { label } => {
                    self.ip = label.0 - 1;
                }
                Instr::Call { label } => {
                    self.call_stack.push(self.ip + 1);
                    self.ip = label.0 - 1;
                }
                Instr::Return {} => match self.call_stack.pop() {
                    Some(label) => {
                        self.ip = label;
                    }
                    None => {
                        // Return in main function
                        return;
                    }
                },
                _ => unimplemented!("TODO"),
            };
            self.ip += 1;
        }
    }
}
