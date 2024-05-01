pub mod ast;
pub mod lexer;
pub mod parser;
pub mod scope;
pub mod tac;

#[cfg(test)]
mod tests {
    use self::lexer::Token;

    use super::*;

    #[test]
    fn lexing() {
        let mut l = lexer::Lexer::new(
            "
    p := 0;
    q := 1;
    while p < 200 {
        t := p + q;
        q = p;
        p = t;
    }"
            .chars()
            .collect(),
        );

        let mut tokens = Vec::new();
        loop {
            let t = l.scan();
            if t == Token::EOF {
                break;
            }
            tokens.push(t);
        }

        assert_eq!(
            tokens,
            vec![
                Token::Word("p".into()),
                Token::DeclAssign,
                Token::Integer(0),
                Token::C(';'),
                Token::Word("q".into()),
                Token::DeclAssign,
                Token::Integer(1),
                Token::C(';'),
                Token::While,
                Token::Word("p".into()),
                Token::C('<'),
                Token::Integer(200),
                Token::C('{'),
                Token::Word("t".into()),
                Token::DeclAssign,
                Token::Word("p".into()),
                Token::C('+'),
                Token::Word("q".into()),
                Token::C(';'),
                Token::Word("q".into()),
                Token::C('='),
                Token::Word("p".into()),
                Token::C(';'),
                Token::Word("p".into()),
                Token::C('='),
                Token::Word("t".into()),
                Token::C(';'),
                Token::C('}')
            ]
        )
    }

    #[test]
    fn parsing() {
        let l = lexer::Lexer::new(
            "
    p := 0;
    q := 1;
    while p < 200 {
        t := p + q;
        q = p;
        p = t;
    }"
            .chars()
            .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        println!("{:?}", prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], tac::DataVal::Integer(233));
        assert_eq!(prog.variables[1], tac::DataVal::Integer(144));
    }

    #[test]
    fn scopes() {
        let l = lexer::Lexer::new(
            "
    p := 5;
    q := 6;
    if true {
        p := 7;
        q := 6.0f;
    }

    r := p;"
                .chars()
                .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        println!("{:?}", prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], tac::DataVal::Integer(5));
        assert_eq!(prog.variables[1], tac::DataVal::Integer(6));
        assert_eq!(prog.variables[2], tac::DataVal::Integer(7));
        assert_eq!(prog.variables[3], tac::DataVal::Float(6.0));
        assert_eq!(prog.variables[4], tac::DataVal::Integer(5));
    }

    #[test]
    fn functions() {
        let l = lexer::Lexer::new(
            "
    func rand() (int) {
        p := 4;
        return p;
    }

    p := rand();

    func test() () {
        idk := 5;
    }

    test();"
                .chars()
                .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        println!("{:?}", prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], tac::DataVal::Integer(4));
        assert_eq!(prog.variables[2], tac::DataVal::Integer(4));
        assert_eq!(prog.variables[3], tac::DataVal::Integer(5));
    }

    #[test]
    fn arrays() {
        let l = lexer::Lexer::new(
            "
    p := 5;
    q := [2, 2, 3, p];
    q[0] = 1;
    p = q[0];"
                .chars()
                .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        println!("{:?}", prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], tac::DataVal::Integer(1))
    }

    #[test]
    fn structs() {
        let l = lexer::Lexer::new(
            "
    struct Test {
        n1: int,
        n2: float
    }

    p := Test{
        n1: 5,
        n2: 6.0f
    };
    q := p.n1;
    r := p.n2;"
                .chars()
                .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        println!("{:?}", prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[1], tac::DataVal::Integer(5));
        assert_eq!(prog.variables[2], tac::DataVal::Float(6.));
    }
}
