mod ast;
mod lexer;
mod parser;
mod symbols;
mod tac;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing() {
        let l = lexer::Lexer::new(
            "
{
    int p ;
    int q ;
    int t ;
    p = 0 ;
    q = 1 ;
    while p < 200 {
        t = p + q ;
        q = p ;
        p = t ;
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
}
