use std::collections::HashMap;

use enum_as_inner::EnumAsInner;

use crate::lexer::{self, Token};
use crate::stac;

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

#[derive(Clone)]
pub struct Struct {
    pub types: Vec<DataType>,
    pub names: HashMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum DataType {
    Integer,
    Float,
    Bool,
    String,
    Array(Box<DataType>),
    Struct(String), // the name of struct
    Function {
        params: Vec<DataType>,
        returns: Vec<DataType>,
    },
    Waiting, // this value is waiting on an external resource to be created
}

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum DataVal {
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Compound(Vec<DataVal>),
    Function(Label),
    Waiting,
}

#[derive(Debug, Clone)]
pub enum Instr {
    BinaryExpr {
        op: lexer::Token,
    },
    Concat,
    UnaryExpr {
        op: lexer::Token,
    },

    LoadConst {
        v: stac::DataVal,
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
    IfEnd {
        // Marks the end of a control flow statement including an if statement.
        // Used when evaluating the side effects of such a statement.
        start: Label,
    },

    Discard, // discards an element from the eval_stack

    CompoundGet,    // arr, index
    CompoundSet,    // arr, index, value
    CompoundCreate, // length

    Goto {
        label: Label, // so far no need for a dynamic goto
    },
    Call,   // Adds the return label to the call stack, then does a goto to the function
    Return, // Pop the previous label on the callstack and goto it

    ExternCall {
        params_count: usize,
    },
}

macro_rules! arith {
    ($self:ident, $op:expr) => {{
        let x = $self.eval_stack.pop().unwrap();
        let y = $self.eval_stack.pop().unwrap();
        if (x.is_waiting() || y.is_waiting()) {
            $self.eval_stack.push(DataVal::Waiting);
        } else {
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
        }
    }};
}

macro_rules! rel {
    ($self:ident, $op:expr) => {{
        let x = $self.eval_stack.pop().unwrap();
        let y = $self.eval_stack.pop().unwrap();
        if (x.is_waiting() || y.is_waiting()) {
            $self.eval_stack.push(DataVal::Waiting);
        } else {
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
        }
    }};
}

// A three address code program
pub struct Prog {
    pub code: Vec<Instr>,

    pub eval_stack: Vec<DataVal>,
    pub variables: Vec<DataVal>,
    pub user_structs: HashMap<String, Struct>,

    ip: usize, // instruction pointer
    call_stack: Vec<usize>,

    evaluating_side_effects: bool,
    if_markers: Vec<Label>,
    false_paths: Vec<Label>,
    pub external_functions: HashMap<String, fn(Vec<DataVal>) -> Vec<DataVal>>,
}

impl Prog {
    pub fn new() -> Prog {
        Prog {
            code: vec![],
            eval_stack: vec![],
            variables: vec![],
            ip: 0,
            call_stack: vec![],
            user_structs: HashMap::new(),
            evaluating_side_effects: false,
            if_markers: vec![],
            false_paths: vec![],
            external_functions: HashMap::new(),
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
                Instr::Concat => {
                    let mut x = self.eval_stack.pop().unwrap().into_string().unwrap();
                    let y = self.eval_stack.pop().unwrap().into_string().unwrap();
                    x.push_str(&y);
                    self.eval_stack.push(DataVal::String(x));
                }
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
                            DataVal::Waiting => self.eval_stack.push(DataVal::Waiting),
                            _ => panic!("'{op}' operator unimplemented for data type"),
                        }
                    }
                    Token::C('!') => {
                        let top = self.eval_stack.pop().unwrap();
                        match top {
                            DataVal::Bool(b) => {
                                self.eval_stack.push(DataVal::Bool(!b));
                            }
                            _ => panic!("'{op}' operator unimplemented for data type"),
                        }
                    }
                    _ => panic!("unimplemented operator '{op}' for unary expression"),
                },
                Instr::LoadConst { v } => self.eval_stack.push(v),
                Instr::LoadIdent { i } => self.eval_stack.push(self.variables[i.0].clone()),
                Instr::StoreIdent { i } => {
                    if self.evaluating_side_effects {
                        self.variables[i.0] = DataVal::Waiting;
                    } else {
                        self.variables[i.0] = self.eval_stack.pop().unwrap()
                    }
                }
                Instr::IfExpr { if_true, if_false } => match self.eval_stack.pop().unwrap() {
                    DataVal::Bool(b) => {
                        if b {
                            if if_true != Label::CONTINUE {
                                self.ip = if_true.0 - 1;
                            }
                        } else {
                            if if_false != Label::CONTINUE {
                                self.ip = if_false.0 - 1;
                            }
                        }
                    }
                    DataVal::Waiting => {
                        println!("if expr at {}", self.ip);

                        // Evaluate side effects of the both paths
                        self.evaluating_side_effects = true;
                        self.if_markers.push(Label(self.ip));

                        if if_true != Label::CONTINUE {
                            self.ip = if_true.0;
                        }
                    }
                    _ => panic!("can only if on bool"),
                },
                Instr::IfEnd { start } => match self.if_markers.last() {
                    Some(im) => {
                        println!("if end at {} from {}", self.ip, start.0);
                        if *im == start {
                            println!(
                                "matching {:?} and fp {:?}",
                                self.if_markers.last().unwrap(),
                                self.false_paths.last()
                            );
                            if self.false_paths.len() == self.if_markers.len() {
                                let fp = self.false_paths.pop().unwrap();
                                if fp == Label::CONTINUE {
                                    self.ip = start.0 - 1;
                                } else {
                                    self.ip = fp.0 - 1;
                                }
                            } else {
                                self.if_markers.pop();

                                if self.if_markers.len() == 0 {
                                    self.evaluating_side_effects = false;
                                }
                            }
                        }
                    }
                    None => {}
                },
                Instr::CompoundGet => {
                    let index = self.eval_stack.pop().unwrap();
                    let arr = self.eval_stack.pop().unwrap();
                    if index.is_waiting() || arr.is_waiting() {
                        self.eval_stack.push(DataVal::Waiting);
                    } else {
                        let val = arr.into_compound().unwrap()
                            [index.into_integer().unwrap() as usize]
                            .clone();
                        self.eval_stack.push(val);
                    }
                }
                Instr::CompoundSet => {
                    let val = self.eval_stack.pop().unwrap();
                    let index = self.eval_stack.pop().unwrap();
                    let arr = self.eval_stack.pop().unwrap();

                    if index.is_waiting() || arr.is_waiting() {
                        self.eval_stack.push(DataVal::Waiting);
                    } else {
                        let mut a = arr.into_compound().unwrap();
                        a[index.into_integer().unwrap() as usize] = val;
                        self.eval_stack.push(DataVal::Compound(a));
                    }
                }
                Instr::CompoundCreate => {
                    let len = self.eval_stack.pop().unwrap();
                    if len.is_waiting() {
                        self.eval_stack.push(DataVal::Waiting);
                    } else {
                        let arr = vec![DataVal::Bool(false); len.into_integer().unwrap() as usize];
                        self.eval_stack.push(DataVal::Compound(arr));
                    }
                }
                Instr::Goto { label } => {
                    self.ip = label.0 - 1;
                }
                Instr::Call => {
                    let f = self.eval_stack.pop().unwrap().into_function().unwrap();
                    self.call_stack.push(self.ip + 1);
                    self.ip = f.0 - 1;
                }
                Instr::Return {} => match self.call_stack.pop() {
                    Some(label) => {
                        self.ip = label - 1;
                    }
                    None => {
                        // Return in main function
                        return;
                    }
                },
                Instr::Discard => {
                    self.eval_stack.pop();
                }
                Instr::ExternCall { params_count } => {
                    let func_name = self.eval_stack.pop().unwrap().into_string().unwrap();

                    let params = self
                        .eval_stack
                        .split_off(self.eval_stack.len() - params_count);

                    let mut returns =
                        self.external_functions
                            .get(&func_name)
                            .expect("unknown external function")(params);

                    self.eval_stack.append(&mut returns);
                }
            };
            self.ip += 1;
        }
    }
}
