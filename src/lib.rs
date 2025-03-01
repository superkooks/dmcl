pub mod ast;
pub mod lexer;
pub mod parser;
pub mod provider;
pub mod scope;
pub mod stac;

#[cfg(test)]
mod tests {
    use stac::DataVal;

    use self::lexer::Token;

    use super::*;

    #[test]
    fn lexing() {
        let mut l = lexer::Lexer::new(
            r#"
    p := 0;
    q := 1;
    while p < 200 {
        t := p + q;
        q = p;
        p = t;
    }
    
    "hello world"
    "#
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
                Token::C('}'),
                Token::String("hello world".into()),
            ]
        )
    }

    #[test]
    fn parsing() {
        let l = lexer::Lexer::new(
            r#"
    p := 0;
    q := 1;
    while p < 200 {
        t := p + q;
        q = p;
        p = t;
    }
    
    k := "hello" + "world";"#
                .chars()
                .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        print_instructions(&prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], stac::DataVal::Integer(233));
        assert_eq!(prog.variables[1], stac::DataVal::Integer(144));
        assert_eq!(
            prog.variables[3],
            stac::DataVal::String("helloworld".into())
        );
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
        print_instructions(&prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], stac::DataVal::Integer(5));
        assert_eq!(prog.variables[1], stac::DataVal::Integer(6));
        assert_eq!(prog.variables[2], stac::DataVal::Integer(7));
        assert_eq!(prog.variables[3], stac::DataVal::Float(6.0));
        assert_eq!(prog.variables[4], stac::DataVal::Integer(5));
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
    rand();

    func test() () {
        idk := 5;
    }

    test();
    
    func huh(test: int) (int) {
        test = test + 1;
        return test;
    }
    
    q := huh(6);
    
    func sub(a: int, b: int) (int) {
        return a-b;
    }
    
    r := sub(9, 10);"
                .chars()
                .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        print_instructions(&prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], stac::DataVal::Integer(4));
        assert_eq!(prog.variables[1], stac::DataVal::Integer(4));
        assert_eq!(prog.variables[2], stac::DataVal::Integer(5));
        assert_eq!(prog.variables[4], stac::DataVal::Integer(7));
        assert_eq!(prog.variables[7], stac::DataVal::Integer(-1));
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
        print_instructions(&prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], stac::DataVal::Integer(1))
    }

    #[test]
    fn structs() {
        let l = lexer::Lexer::new(
            "
    struct Test {
        n1: int,
        n2: float
        n3: float
    }

    p := Test{
        n1: 5,
        n2: 6.0f
    };
    q := p.n1;
    r := p.n2;
    w := p.n3;"
                .chars()
                .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        print_instructions(&prog.code);

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[1], stac::DataVal::Integer(5));
        assert_eq!(prog.variables[2], stac::DataVal::Float(6.));
        assert_eq!(prog.variables[3], stac::DataVal::Float(0.));
    }

    #[test]
    fn extern_func() {
        let l = lexer::Lexer::new(
            r#"
    func extern createResource(name: string) (int)
    func extern createResourceAsync(name: string) (int)

    p := createResource("test");
    q := createResourceAsync("test3");

    func test() (int) {
        p := 1;
        if p == 1 {
            p = 2;
        }
        return p;
    }

    a := 1;
    if q < 1 {
        a = 2;
    } else {
        a = 3;
    }

    b := test();
    c := 1;
    if q < 1 {
        c = test();
    }
    "#
            .chars()
            .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        print_instructions(&prog.code);

        prog.external_functions.insert(
            "createResource".into(),
            Box::new(|_ip, _ptype, _rtypes, params, _ustructs| {
                println!(
                    "creating resource {}",
                    params[0].clone().into_string().unwrap()
                );
                return vec![DataVal::Integer(6)];
            }),
        );

        prog.external_functions.insert(
            "createResourceAsync".into(),
            Box::new(|_ip, _ptype, _rtypes, params, _ustructs| {
                println!(
                    "creating resource asynchronously {}",
                    params[0].clone().into_string().unwrap()
                );
                return vec![DataVal::Waiting];
            }),
        );

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], stac::DataVal::Integer(6));
        assert_eq!(prog.variables[1], stac::DataVal::Waiting);
        assert_eq!(prog.variables[2], stac::DataVal::Waiting);
        assert_eq!(prog.variables[4], stac::DataVal::Integer(2));
        assert_eq!(prog.variables[5], stac::DataVal::Waiting);
    }

    #[test]
    fn extern_func2() {
        let l = lexer::Lexer::new(
            r#"
    func extern createResourceAsync(name: string) (int)

    q := createResourceAsync("test3");

    a := 1;
    if q == 0 {
        if q == 0 {
            a = 2;
        }
        a = 3;
    }

    b := 2;
    while a == 1 {
        b = 3;
    }
    "#
            .chars()
            .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        print_instructions(&prog.code);

        prog.external_functions.insert(
            "createResourceAsync".into(),
            Box::new(|_ip, _ptype, _rtypes, params, _ustructs| {
                println!(
                    "creating resource asynchronously {}",
                    params[0].clone().into_string().unwrap()
                );
                return vec![DataVal::Waiting];
            }),
        );

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[0], stac::DataVal::Waiting);
        assert_eq!(prog.variables[1], stac::DataVal::Waiting);
        assert_eq!(prog.variables[2], stac::DataVal::Waiting);
    }

    #[test]
    fn fake_provider() {
        let l = lexer::Lexer::new(
            r#"
    struct CreateDropletRequest {
        name: string,
        region: string,
        size: string,
        image: string,
        ssh_keys: []string,
        backups: bool,
        ipv6: bool,
        monitoring: bool,
        tags: []string,
        user_data: string,
        volumes: []string,
        vpc_uuid: string,
        with_droplet_agent: bool
    }

    struct DropletNetwork {
        ip_address: string,
        netmask: string,
        gateway: string,
        type: string
    }

    struct DropletNetworking {
        v4: []DropletNetwork,
        v6: []DropletNetwork
    }

    struct Droplet {
        id: int,
        state: string,
        name: string,
        networks: DropletNetworking
    }

    func extern createDroplet(req: CreateDropletRequest) (Droplet)

    q := createDroplet(CreateDropletRequest {
        name: "hello-world",
        region: "syd1",
        size: "s-1vcpu-512mb-10gb",
        image: "ubuntu-20-04-x64"
    });
    c := q.state;
    "#
            .chars()
            .collect(),
        );

        let mut par = parser::Parser::new(l);
        let prog = par.program();
        print_instructions(&prog.code);

        prog.add_http_provider("http://localhost:8080".into());

        prog.execute();
        println!("{:?}", prog.variables);

        assert_eq!(prog.variables[1], stac::DataVal::Waiting)
    }
}

pub fn print_instructions(blocks: &Vec<stac::Block>) {
    for (k, block) in blocks.iter().enumerate() {
        println!("\nBLOCK {:3}:", k);
        for (k, instr) in block.code.iter().enumerate() {
            println!("{:3}:  {:?}", k, instr)
        }
    }
}
