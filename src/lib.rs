mod lexer;
mod symbols;
mod tac;

fn emit(s: &str) {
    println!("{}", s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing() {
        let mut p = lexer::Lexer::new("p = 1 + 1\n".chars().collect());
        loop {
            println!("{:?}", p.scan());
        }
    }
}
