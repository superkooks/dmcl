pub mod ast;
pub mod lexer;
pub mod parser;
pub mod symbols;
pub mod tac;

#[cfg(test)]
mod tests {
    use self::lexer::Token;

    use super::*;

    #[test]
    fn lexing() {
        let mut l = lexer::Lexer::new(
            "
{
    int p;
    int q;
    int t;
    p = 0;
    q = 1;
    while p < 200 {
        t = p + q;
        q = p;
        p = t;
    }
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
                Token::C('{'),
                Token::Type(tac::DataType::Integer(0)),
                Token::Word("p".into()),
                Token::C(';'),
                Token::Type(tac::DataType::Integer(0)),
                Token::Word("q".into()),
                Token::C(';'),
                Token::Type(tac::DataType::Integer(0)),
                Token::Word("t".into()),
                Token::C(';'),
                Token::Word("p".into()),
                Token::C('='),
                Token::Integer(0),
                Token::C(';'),
                Token::Word("q".into()),
                Token::C('='),
                Token::Integer(1),
                Token::C(';'),
                Token::While,
                Token::Word("p".into()),
                Token::C('<'),
                Token::Integer(200),
                Token::C('{'),
                Token::Word("t".into()),
                Token::C('='),
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
                Token::C('}'),
                Token::C('}')
            ]
        )
    }

    #[test]
    fn parsing() {
        let l = lexer::Lexer::new(
            "
{
    int p;
    int q;
    int t;
    p = 0;
    q = 1;
    while p < 200 {
        t = p + q;
        q = p;
        p = t;
    }
}"
            .chars()
            .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        println!("{:?}", prog.code);

        prog.execute();
        println!("{:?}", prog.memory);

        assert_eq!(prog.memory[0], tac::DataType::Integer(233));
        assert_eq!(prog.memory[1], tac::DataType::Integer(144));
    }

    #[test]
    fn scopes() {
        let l = lexer::Lexer::new(
            "
{
    int p;
    int q;
    int r;
    p = 5;
    q = 6;
    if true {
        int p;
        float q;
        p = 7;
        q = 6.0f;
    }

    r = p;
}"
            .chars()
            .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        println!("{:?}", prog.code);

        prog.execute();
        println!("{:?}", prog.memory);

        assert_eq!(prog.memory[0], tac::DataType::Integer(5));
        assert_eq!(prog.memory[1], tac::DataType::Integer(6));
        assert_eq!(prog.memory[3], tac::DataType::Integer(7));
        assert_eq!(prog.memory[4], tac::DataType::Float(6.0));
        assert_eq!(prog.memory[2], tac::DataType::Integer(5));
    }
}
