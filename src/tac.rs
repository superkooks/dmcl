use crate::lexer::{self, Token};

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
        c: lexer::Token,
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataType {
    Integer(i64),
    Float(f64),
    Bool(bool),
}

macro_rules! get_int {
    ($from:expr) => {
        match $from {
            DataType::Integer(i) => i,
            _ => panic!("type error"),
        }
    };
}

macro_rules! get_float {
    ($from:expr) => {
        match $from {
            DataType::Float(f) => f,
            _ => panic!("type error"),
        }
    };
}

macro_rules! get_bool {
    ($from:expr) => {
        match $from {
            DataType::Bool(b) => b,
            _ => panic!("type error"),
        }
    };
}

macro_rules! arith {
    ($self:ident, $op:expr, $to:ident, $x:ident, $y:ident) => {
        match $self.memory[$x.0] {
            DataType::Integer(_) => {
                $self.memory[$to.0] = DataType::Integer($op(
                    get_int!($self.memory[$x.0]),
                    get_int!($self.memory[$y.0]),
                ))
            }
            DataType::Float(_) => {
                $self.memory[$to.0] = DataType::Float($op(
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
            DataType::Integer(_) => {
                $self.memory[$to.0] = DataType::Bool($op(
                    &get_int!($self.memory[$x.0]),
                    &get_int!($self.memory[$y.0]),
                ))
            }
            DataType::Float(_) => {
                $self.memory[$to.0] = DataType::Bool($op(
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
    pub memory: Vec<DataType>,
    pub code: Vec<Instr>,

    ip: usize, // instruction pointer
}

impl Prog {
    pub fn new() -> Prog {
        Prog {
            memory: Vec::new(),
            code: Vec::new(),
            ip: 0,
        }
    }

    pub fn allocate_var(&mut self) -> Addr {
        // Doesn't matter what we set it to, just return the address
        self.memory.push(DataType::Bool(false));
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

                    Token::C('=') => self.memory[to.0] = self.memory[x.0],
                    _ => panic!("unimplemented operator"),
                },
                Instr::StoreConst { c, addr } => match c {
                    Token::Integer(i) => self.memory[addr.0] = DataType::Integer(i),
                    Token::Float(f) => self.memory[addr.0] = DataType::Float(f),
                    Token::True => self.memory[addr.0] = DataType::Bool(true),
                    Token::False => self.memory[addr.0] = DataType::Bool(false),
                    _ => panic!("invalid constant"),
                },
                Instr::Goto { label } => {
                    self.ip = label.0;
                }
                Instr::IfExpr {
                    test,
                    if_true,
                    if_false,
                } => match self.memory[test.0] {
                    DataType::Bool(b) => {
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
            };
            self.ip += 1;
        }
    }
}
