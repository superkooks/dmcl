use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering::Relaxed;

use crate::{emit, lexer};

#[derive(Clone, Copy)]
pub enum DataType {
    Integer,
    Float,
    Bool,
    // string
    // composite types
}

trait ExprAble {
    fn gen(self: Box<Self>) -> Box<dyn ExprAble>;
    fn reduce(self: Box<Self>) -> Box<dyn ExprAble>;
    fn to_string(&self) -> String;
}

pub struct Ident {
    id: lexer::Token,
    data_type: DataType,
}

impl ExprAble for Ident {
    fn gen(self: Box<Self>) -> Box<dyn ExprAble> {
        return self;
    }

    fn reduce(self: Box<Self>) -> Box<dyn ExprAble> {
        return self;
    }

    fn to_string(&self) -> String {
        return self.id.to_string();
    }
}

pub struct Arith {
    x: Box<dyn ExprAble>,
    y: Box<dyn ExprAble>,
    op: lexer::Token,
    data_type: DataType,
}

impl ExprAble for Arith {
    fn gen(self: Box<Self>) -> Box<dyn ExprAble> {
        return Box::new(Arith {
            x: self.x.reduce(),
            y: self.y.reduce(),
            op: self.op,
            data_type: self.data_type,
        });
    }

    fn reduce(self: Box<Self>) -> Box<dyn ExprAble> {
        let d = self.data_type;
        let x = self.gen();
        let t = Temp::new(d);
        emit(format!("{} = {}", t.to_string(), x.to_string()).as_str());
        return t;
    }

    fn to_string(&self) -> String {
        return format!(
            "{} {} {}",
            self.x.to_string(),
            self.op.to_string(),
            self.y.to_string()
        );
    }
}

pub struct Unary {
    x: Box<dyn ExprAble>,
    op: lexer::Token,
    data_type: DataType,
}

impl ExprAble for Unary {
    fn gen(self: Box<Self>) -> Box<dyn ExprAble> {
        return Box::new(Unary {
            x: self.x.reduce(),
            op: self.op,
            data_type: self.data_type,
        });
    }

    fn reduce(self: Box<Self>) -> Box<dyn ExprAble> {
        let d = self.data_type;
        let x = self.gen();
        let t = Temp::new(d);
        emit(format!("{} = {}", t.to_string(), x.to_string()).as_str());
        return t;
    }

    fn to_string(&self) -> String {
        return format!("{} {}", self.op.to_string(), self.x.to_string());
    }
}

static TEMP_COUNT: AtomicI64 = AtomicI64::new(0);

pub struct Temp {
    data_type: DataType,
    number: i64,
}

impl Temp {
    fn new(data_type: DataType) -> Box<Self> {
        let t = Temp {
            number: TEMP_COUNT.load(Relaxed),
            data_type,
        };
        TEMP_COUNT.fetch_add(1, Relaxed);
        return Box::new(t);
    }
}

impl ExprAble for Temp {
    fn gen(self: Box<Self>) -> Box<dyn ExprAble> {
        return self;
    }

    fn reduce(self: Box<Self>) -> Box<dyn ExprAble> {
        return self;
    }

    fn to_string(&self) -> String {
        return format!("t{}", self.number);
    }
}

pub struct Const {
    token: lexer::Token,
}

impl Const {
    fn jumping(self: Box<Self>, t: i64, f: i64) {
        if self.token == lexer::Token::Word("true".to_string()) && t != 0 {
            emit(format!("goto L{}", t).as_str());
        } else if self.token == lexer::Token::Word("false".to_string()) && f != 0 {
            emit(format!("goto L{}", f).as_str());
        }
    }
}

impl ExprAble for Const {
    fn gen(self: Box<Self>) -> Box<dyn ExprAble> {
        return self;
    }

    fn reduce(self: Box<Self>) -> Box<dyn ExprAble> {
        return self;
    }

    fn to_string(&self) -> String {
        return self.token.to_string();
    }
}

pub struct Or {
    x: Box<dyn ExprAble>,
    y: Box<dyn ExprAble>,
}

impl ExprAble for Or {
    fn gen(self: Box<Self>) -> Box<dyn ExprAble> {}

    fn reduce(self: Box<Self>) -> Box<dyn ExprAble> {
        return self;
    }

    fn to_string(&self) -> String {
        return format!("{} {} {}", self.x.to_string(), self);
    }
}
