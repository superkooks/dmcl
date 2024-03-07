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
    AssignExpr {
        // to = x (op) y
        op: lexer::Token,
        to: Addr,
        x: Addr,
        y: Addr, // if y = 0, then this is an unary instruction
    },

    StoreConst {
        // Stores the constant to addr
        v: tac::DataVal,
        addr: Addr,
    },

    IfExpr {
        // if (test) then goto (if_true)
        test: Addr, // must point to bool

        // Special label CONTINUE indicates continuation of execution
        if_true: Label,
        if_false: Label,
    },

    Goto {
        label: Label,
    },

    ArrayGet {
        index: Addr,
        arr: Addr,
        to: Addr,
    },

    ArraySet {
        index: Addr,
        arr: Addr,
        from: Addr,
    },

    ArrayCreate {
        arr: Addr,
        count: Addr,
    },
    Call {
        // Sets the return address on the call stack, then does a goto
        label: Label,
    },

    Return {},
}

#[derive(Clone, Debug, PartialEq)]
pub enum DataType {
    Integer,
    Float,
    Bool,
    Compound(Box<DataType>),
    Func {
        params: Vec<DataType>,
        returns: Vec<DataType>,
    },
}

// Dataval for func:
// Func {
//     params_mem: Vec<Addr>, // where should the params be stored when calling this function
//     returns_mem: Vec<Addr>, // where the returned variables will be stored
// }

#[derive(Clone, Debug, PartialEq)]
pub enum DataVal {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Compound(Vec<DataVal>),
}

macro_rules! get_int {
    ($from:expr) => {
        match $from {
            DataVal::Integer(i) => i,
            _ => panic!("type error"),
        }
    };
}

macro_rules! get_float {
    ($from:expr) => {
        match $from {
            DataVal::Float(f) => f,
            _ => panic!("type error"),
        }
    };
}

macro_rules! get_bool {
    ($from:expr) => {
        match $from {
            DataVal::Bool(b) => b,
            _ => panic!("type error"),
        }
    };
}

macro_rules! arith {
    ($self:ident, $op:expr, $to:ident, $x:ident, $y:ident) => {
        match $self.memory[$x.0] {
            DataVal::Integer(_) => {
                $self.memory[$to.0] = DataVal::Integer($op(
                    get_int!($self.memory[$x.0]),
                    get_int!($self.memory[$y.0]),
                ))
            }
            DataVal::Float(_) => {
                $self.memory[$to.0] = DataVal::Float($op(
                    get_float!($self.memory[$x.0]),
                    get_float!($self.memory[$y.0]),
                ))
            }
            _ => panic!("cannot use arithmetic on those types"),
        }
    };
}

macro_rules! rel {
    ($self:ident, $op:expr, $to:ident, $x:ident, $y:ident) => {
        match $self.memory[$x.0] {
            DataVal::Integer(_) => {
                $self.memory[$to.0] = DataVal::Bool($op(
                    &get_int!($self.memory[$x.0]),
                    &get_int!($self.memory[$y.0]),
                ))
            }
            DataVal::Float(_) => {
                $self.memory[$to.0] = DataVal::Bool($op(
                    &get_float!($self.memory[$x.0]),
                    &get_float!($self.memory[$y.0]),
                ))
            }
            _ => panic!("cannot compare those types"),
        }
    };
}

// A three address code program
pub struct Prog {
    pub memory: Vec<DataVal>,
    pub code: Vec<Instr>,

    ip: usize, // instruction pointer
    call_stack: Vec<usize>,
}

impl Prog {
    pub fn new() -> Prog {
        Prog {
            memory: Vec::new(),
            code: Vec::new(),
            ip: 0,
            call_stack: Vec::new(),
        }
    }

    pub fn allocate_var(&mut self) -> Addr {
        // Doesn't matter what we set it to, just return the address
        self.memory.push(DataVal::Bool(false));
        return Addr(self.memory.len() - 1);
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
                Instr::AssignExpr { op, to, x, y } => match op {
                    Token::C('+') => arith!(self, std::ops::Add::add, to, x, y),
                    Token::C('-') if y.0 != 0 => arith!(self, std::ops::Sub::sub, to, x, y),
                    Token::C('*') => arith!(self, std::ops::Mul::mul, to, x, y),
                    Token::C('/') => arith!(self, std::ops::Div::div, to, x, y),

                    Token::Eq => rel!(self, std::cmp::PartialEq::eq, to, x, y),
                    Token::Ne => rel!(self, std::cmp::PartialEq::ne, to, x, y),

                    Token::C('<') => rel!(self, std::cmp::PartialOrd::lt, to, x, y),
                    Token::Le => rel!(self, std::cmp::PartialOrd::le, to, x, y),
                    Token::C('>') => rel!(self, std::cmp::PartialOrd::gt, to, x, y),
                    Token::Ge => rel!(self, std::cmp::PartialOrd::ge, to, x, y),

                    Token::C('=') => self.memory[to.0] = self.memory[x.0].clone(),
                    _ => panic!("unimplemented operator"),
                },
                Instr::StoreConst { v, addr } => {
                    self.memory[addr.0] = v;
                }
                Instr::IfExpr {
                    test,
                    if_true,
                    if_false,
                } => match self.memory[test.0] {
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
                Instr::ArrayGet { index, arr, to } => match self.memory[arr.0].clone() {
                    DataVal::Compound(vals) => match self.memory[index.0].clone() {
                        DataVal::Integer(index) => {
                            self.memory[to.0] = vals[index as usize].clone();
                        }
                        _ => panic!("can only index compound types by integer"),
                    },
                    _ => panic!("can only index compound types"),
                },
                Instr::ArraySet { index, arr, from } => match self.memory[arr.0].clone() {
                    DataVal::Compound(mut vals) => match self.memory[index.0].clone() {
                        DataVal::Integer(index) => {
                            vals[index as usize] = self.memory[from.0].clone();
                            self.memory[arr.0] = DataVal::Compound(vals);
                        }
                        _ => panic!("can only index compound types by integer"),
                    },
                    _ => panic!("can only index compound types"),
                },
                Instr::ArrayCreate { arr, count } => {
                    let len = get_int!(self.memory[count.0]);
                    let mut temp = Vec::with_capacity(len as usize);

                    for _ in 0..len {
                        temp.push(DataVal::Bool(false));
                    }

                    self.memory[arr.0] = DataVal::Compound(temp);
                }
                Instr::Goto { label } => {
                    self.ip = label.0;
                }
                Instr::Call { label } => {
                    self.call_stack.push(self.ip + 1);
                    self.ip = label.0;
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
            };
            self.ip += 1;
        }
    }
}
