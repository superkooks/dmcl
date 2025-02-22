use std::collections::HashMap;

use enum_as_inner::EnumAsInner;
use serde::de::DeserializeSeed;

use crate::lexer::{self, Token};
use crate::provider::{ExternReturns, ProviderSchema, TypeAndVal, DMCLRPC};
use crate::stac;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Addr(pub usize); // Addr of variable in memory

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label(pub usize); // A label of a block to jump to.

impl Label {
    pub const CONTINUE: Label = Label(usize::MAX); // continue execution. used in if.
}

#[derive(Clone)]
pub struct Struct {
    pub types: Vec<DataType>,
    pub names: HashMap<String, usize>,
}

#[derive(Clone)]
pub struct Function {
    pub label: Label,
    pub params: Vec<DataType>,
    pub returns: Vec<DataType>,
}

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum DataType {
    Integer,
    Float,
    Bool,
    String,
    Array(Box<DataType>),
    Struct(String), // the name of struct
    Waiting,        // this value is waiting on an external resource to be created
}

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum DataVal {
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Compound(Vec<DataVal>),
    Waiting,
}

impl DataVal {
    pub fn default_for(ty: DataType, user_structs: &HashMap<String, Struct>) -> Self {
        match ty {
            DataType::Integer => DataVal::Integer(0),
            DataType::Float => DataVal::Float(0.0),
            DataType::Bool => DataVal::Bool(false),
            DataType::String => DataVal::String("".into()),
            DataType::Array(_) => DataVal::Compound(vec![]),
            DataType::Struct(struct_name) => {
                let strct = user_structs.get(&struct_name).unwrap();
                let mut compound = vec![DataVal::Bool(false); strct.names.len()];
                for (_, idx) in &strct.names {
                    // Get the default value for the type
                    compound[*idx] = DataVal::default_for(strct.types[*idx].clone(), user_structs);
                }

                DataVal::Compound(compound)
            }
            DataType::Waiting => panic!("no default value for waiting"),
        }
    }
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

    Discard, // discards an element from the eval_stack

    CompoundGet,    // arr, index
    CompoundSet,    // arr, index, value
    CompoundCreate, // length

    Goto {
        label: Label,
    },
    Call {
        // Adds the return label to the call stack, then does a goto to the function
        label: Label,
    },
    Return, // Pop the previous label on the callstack and goto it

    ExternCall {
        param_types: Vec<DataType>,
        return_types: Vec<DataType>,
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

pub struct Block {
    pub code: Vec<Instr>,
}

impl Block {
    pub fn new() -> Self {
        return Self { code: vec![] };
    }

    pub fn add_instr(&mut self, instr: Instr) {
        self.code.push(instr);
    }
}

pub struct Prog {
    pub code: Vec<Block>,
    pub entrypoint: Label,

    pub eval_stack: Vec<DataVal>,
    pub variables: Vec<DataVal>,
    pub user_structs: HashMap<String, Struct>,
    pub user_functions: HashMap<String, Function>,

    ip: (usize, usize), // instruction pointer (block, instr)
    call_stack: Vec<(usize, usize)>,
    cycles: usize,

    evaluating_side_effects: bool,
    blocks_to_eval: Vec<Label>,
    pub external_functions: HashMap<
        String,
        Box<
            dyn Fn(
                (usize, usize, usize),
                Vec<DataType>,
                Vec<DataType>,
                Vec<DataVal>,
                &HashMap<String, Struct>,
            ) -> Vec<DataVal>,
        >,
    >,
    extern_func_call_count: HashMap<String, usize>,
}

impl Prog {
    pub fn new() -> Prog {
        Prog {
            code: vec![],
            entrypoint: Label(0),
            eval_stack: vec![],
            variables: vec![],
            ip: (0, 0),
            cycles: 0,
            call_stack: vec![],
            user_structs: HashMap::new(),
            user_functions: HashMap::new(),
            evaluating_side_effects: false,
            blocks_to_eval: vec![],
            external_functions: HashMap::new(),
            extern_func_call_count: HashMap::new(),
        }
    }

    pub fn allocate_var(&mut self) -> Addr {
        // Doesn't matter what we set it to, just return the address
        self.variables.push(DataVal::Bool(false));
        return Addr(self.variables.len() - 1);
    }

    pub fn add_block(&mut self, block: Block) -> Label {
        self.code.push(block);
        return Label(self.code.len() - 1);
    }

    pub fn add_temp_block(&mut self) -> Label {
        self.code.push(stac::Block::new());
        return Label(self.code.len() - 1);
    }

    pub fn mod_block(&mut self, block: Block, label: Label) {
        self.code[label.0] = block;
    }

    pub fn add_http_provider(&mut self, addr: String) {
        let schema: ProviderSchema = reqwest::blocking::get(addr.clone() + "/provider_schema")
            .unwrap()
            .json()
            .unwrap();

        for func in schema.functions {
            println!("adding {} from {}", &addr, &func);
            self.add_http_extern(addr.clone(), func);
        }
    }

    pub fn add_http_extern(&mut self, addr: String, name: String) {
        self.external_functions.insert(
            name.clone(),
            Box::new(
                move |id, param_types, return_types, param_vals, user_structs| {
                    let to_ser: Vec<_> = param_types
                        .iter()
                        .enumerate()
                        .map(|(idx, dtype)| TypeAndVal {
                            typ: dtype.clone(),
                            val: param_vals[idx].clone(),
                            user_structs,
                        })
                        .collect();

                    let client = reqwest::blocking::Client::new();
                    let resp = client
                        .post(format!("{}/{}", &addr, &name))
                        .json(&DMCLRPC { id, params: to_ser })
                        .send()
                        .unwrap();
                    let mut deserializer = serde_json::Deserializer::from_reader(resp);

                    let ext_ret = ExternReturns {
                        user_structs,
                        types: return_types,
                    };
                    DeserializeSeed::deserialize(ext_ret, &mut deserializer).unwrap()
                },
            ),
        );
    }

    pub fn execute(&mut self) {
        self.ip = (self.entrypoint.0, 0);

        'outer: loop {
            let instr;
            if self.ip.1 >= self.code[self.ip.0].code.len() {
                if self.ip.0 == self.entrypoint.0 {
                    break;
                }

                instr = Instr::Return
            } else {
                instr = self.code[self.ip.0].code[self.ip.1].clone()
            }

            println!("executing @ {:?} : {:?}", self.ip, instr);

            self.cycles += 1;
            if self.cycles > 1000 {
                break;
            }

            if self.evaluating_side_effects {
                while self.ip.1 >= self.code[self.ip.0].code.len() {
                    match self.blocks_to_eval.pop() {
                        Some(next) => {
                            if next != Label::CONTINUE {
                                self.ip = (next.0, 0);
                                continue 'outer;
                            }
                        }
                        None => {
                            // Stop evaluating side effects
                            self.evaluating_side_effects = false;
                            println!("EXITING side effect mode");
                            self.ip = self.call_stack.pop().unwrap();
                        }
                    }
                }

                match instr {
                    Instr::StoreIdent { i } => {
                        self.variables[i.0] = DataVal::Waiting;
                    }
                    Instr::IfExpr { if_true, if_false } => {
                        self.blocks_to_eval.push(if_true);
                        self.blocks_to_eval.push(if_false);
                    }
                    Instr::Goto { label } => {
                        self.blocks_to_eval.push(label);
                    }
                    Instr::Call { label } => {
                        self.blocks_to_eval.push(label);
                    }
                    _ => {}
                }
            } else {
                match instr {
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
                    Instr::StoreIdent { i } => self.variables[i.0] = self.eval_stack.pop().unwrap(),
                    Instr::IfExpr { if_true, if_false } => match self.eval_stack.pop().unwrap() {
                        DataVal::Bool(b) => {
                            if b {
                                if if_true != Label::CONTINUE {
                                    self.call_stack.push(self.ip);
                                    self.ip = (if_true.0, 0);
                                    continue;
                                }
                            } else {
                                if if_false != Label::CONTINUE {
                                    self.call_stack.push(self.ip);
                                    self.ip = (if_false.0, 0);
                                    continue;
                                }
                            }
                        }
                        DataVal::Waiting => {
                            println!(
                                "if expr at {:?} is waiting, going to side effect mode",
                                self.ip
                            );

                            // Evaluate side effects of both paths
                            self.evaluating_side_effects = true;
                            self.call_stack.push(self.ip);
                            self.ip = (if_true.0, 0);
                            self.blocks_to_eval.push(if_false);
                            continue;
                        }
                        _ => panic!("can only if on bool"),
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
                            let arr =
                                vec![DataVal::Bool(false); len.into_integer().unwrap() as usize];
                            self.eval_stack.push(DataVal::Compound(arr));
                        }
                    }
                    Instr::Goto { label } => {
                        self.call_stack.push(self.ip);
                        self.ip = (label.0, 0);
                        continue;
                    }
                    Instr::Call { label } => {
                        self.call_stack.push(self.ip);
                        self.ip = (label.0, 0);
                        continue;
                    }
                    Instr::Return {} => match self.call_stack.pop() {
                        Some(label) => {
                            self.ip = label;
                            // don't continue, increment past the origin label
                        }
                        None => {
                            // Return in main function
                            return;
                        }
                    },
                    Instr::Discard => {
                        self.eval_stack.pop();
                    }
                    Instr::ExternCall {
                        param_types,
                        return_types,
                    } => {
                        let func_name = self.eval_stack.pop().unwrap().into_string().unwrap();

                        let param_vals = self
                            .eval_stack
                            .split_off(self.eval_stack.len() - param_types.len());

                        let call_site = *self.call_stack.last().unwrap();
                        let call_count = *self.extern_func_call_count.get(&func_name).unwrap_or(&0);

                        let mut returns = self
                            .external_functions
                            .get(&func_name)
                            .expect("unknown external function")(
                            (call_site.0, call_site.1, call_count),
                            param_types,
                            return_types,
                            param_vals,
                            &self.user_structs,
                        );

                        self.eval_stack.append(&mut returns);

                        self.extern_func_call_count
                            .insert(func_name, call_count + 1);
                    }
                }
            };
            self.ip.1 += 1;
        }
    }
}
