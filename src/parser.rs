use crate::ast::{
    BooleanLiteral, Expression, ExpressionStatement, Identifier, InfixExpression, InfixOperator,
    IntegerLiteral, LetStatement, PrefixExpression, PrefixOperator, Program, ReturnStatement,
    Statement,
};
use crate::lexer::Lexer;
use crate::token::Token;

pub struct Parser {
    l: Lexer,
    cur: Token,
    peek: Token,
    errors: Vec<String>,
}

#[derive(Eq, PartialEq, PartialOrd, Ord)]
enum Precedence {
    Lowest = 0,
    Equals = 1,
    LessGreater = 2,
    Sum = 3,
    Product = 4,
    Prefix = 5,
    Call = 6,
}

impl Parser {
    pub fn new(mut l: Lexer) -> Self {
        let cur = l.next_token();
        let peek = l.next_token();
        let errors = Vec::new();
        Parser {
            l,
            cur,
            peek,
            errors,
        }
    }

    pub fn parse(&mut self) -> Program {
        let mut res: Vec<Statement> = Vec::new();
        while self.cur != Token::Eof {
            let stmt = self.parse_statement();
            match stmt {
                Some(s) => res.push(s),
                None => {}
            }
            self.next_token();
        }
        Program { statements: res }
    }

    pub fn errors_len(&self) -> usize {
        self.errors.len()
    }

    pub fn get_errors(&self) -> Vec<String> {
        self.errors.clone()
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        match &self.cur {
            Token::Let => self.parse_let_statement(),
            Token::Return => self.parse_return_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> Option<Statement> {
        let tok = self.cur.clone();
        let name: Identifier;
        if let Token::Ident(v) = self.peek.clone() {
            self.next_token();
            name = Identifier {
                tok: self.cur.clone(),
                value: v.clone(),
            }
        } else {
            let e = format!(
                "expected next token to be Token::Ident, got {:#?} instead",
                self.peek
            );
            self.errors.push(e);
            return None;
        }
        if !self.expect_peek(Token::Assign) {
            return None;
        }
        while !self.cur_token_is(Token::Semicolon) {
            self.next_token();
        }
        Some(Statement::LetStatement(LetStatement { tok, name }))
    }

    fn parse_return_statement(&mut self) -> Option<Statement> {
        let tok = self.cur.clone();
        self.next_token();
        while !self.cur_token_is(Token::Semicolon) {
            self.next_token();
        }
        Some(Statement::ReturnStatement(ReturnStatement { tok }))
    }

    fn parse_expression_statement(&mut self) -> Option<Statement> {
        let tok = self.cur.clone();
        match self.parse_expression(Precedence::Lowest) {
            Some(e) => {
                let res = Some(Statement::ExpressionStatement(ExpressionStatement {
                    tok,
                    expression: e,
                }));
                if self.peek_token_is(&Token::Semicolon) {
                    self.next_token();
                }
                res
            }
            None => None,
        }
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Option<Expression> {
        let mut left = match &self.cur {
            Token::Ident(_) => Some(self.parse_identifier()),
            Token::Int(_) => self.parse_integer_literal(),
            Token::Bang | Token::Minus => self.parse_prefix_expression(),
            Token::True | Token::False => Some(self.parse_boolean_literal()),
            Token::LParen => self.parse_grouped_expression(),
            _ => {
                let e = format!("no prefix parse fn for {:#?}", self.cur);
                self.errors.push(e);
                None
            }
        };

        while !self.peek_token_is(&Token::Semicolon) && precedence < self.peek_precedence() {
            match &self.peek {
                Token::Plus
                | Token::Minus
                | Token::Slash
                | Token::Asterisk
                | Token::Eq
                | Token::NotEq
                | Token::Lt
                | Token::Gt => {
                    self.next_token();
                    let l = match left {
                        Some(exp) => exp,
                        None => return None,
                    };
                    left = self.parse_infix_expression(l);
                }
                _ => return left,
            }
        }
        left
    }

    fn parse_identifier(&mut self) -> Expression {
        if let Token::Ident(v) = &self.cur {
            let tok = self.cur.clone();
            Expression::Identifier(Identifier {
                tok,
                value: v.clone(),
            })
        } else {
            panic!("unreachable");
        }
    }

    fn parse_integer_literal(&mut self) -> Option<Expression> {
        if let Token::Int(v) = &self.cur {
            let tok = self.cur.clone();
            match v.parse::<i64>() {
                Ok(i) => Some(Expression::Integer(IntegerLiteral { tok, value: i })),
                Err(_) => None,
            }
        } else {
            panic!("unreachable");
        }
    }

    fn parse_boolean_literal(&mut self) -> Expression {
        let tok = self.cur.clone();
        let value = self.cur == Token::True;
        Expression::Boolean(BooleanLiteral { tok, value })
    }

    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let operator = match self.cur {
            Token::Minus => PrefixOperator::Minus,
            Token::Bang => PrefixOperator::Bang,
            _ => return None,
        };
        let tok = self.cur.clone();
        self.next_token();
        let right = self.parse_expression(Precedence::Prefix);
        match right {
            Some(exp) => Some(Expression::PrefixExpression(PrefixExpression {
                tok,
                operator,
                right: std::rc::Rc::new(exp),
            })),
            None => None,
        }
    }

    fn parse_infix_expression(&mut self, left: Expression) -> Option<Expression> {
        let operator = match self.cur {
            Token::Plus => InfixOperator::Plus,
            Token::Minus => InfixOperator::Minus,
            Token::Asterisk => InfixOperator::Asterisk,
            Token::Slash => InfixOperator::Slash,
            Token::Eq => InfixOperator::Eq,
            Token::NotEq => InfixOperator::NotEq,
            Token::Lt => InfixOperator::Lt,
            Token::Gt => InfixOperator::Gt,
            _ => return None,
        };
        let tok = self.cur.clone();
        let precedence = self.cur_precedence();
        self.next_token();
        let right = self.parse_expression(precedence);
        match right {
            Some(exp) => Some(Expression::InfixExpression(InfixExpression {
                tok,
                left: std::rc::Rc::new(left),
                operator,
                right: std::rc::Rc::new(exp),
            })),
            None => None,
        }
    }

    fn parse_grouped_expression(&mut self) -> Option<Expression> {
        self.next_token();
        let exp = self.parse_expression(Precedence::Lowest);
        if !self.expect_peek(Token::RParen) {
            return None;
        }
        exp
    }

    fn next_token(&mut self) {
        self.cur = self.peek.clone();
        self.peek = self.l.next_token();
    }

    fn cur_token_is(&self, tok: Token) -> bool {
        self.cur == tok
    }

    fn peek_token_is(&self, tok: &Token) -> bool {
        self.peek == *tok
    }

    fn expect_peek(&mut self, tok: Token) -> bool {
        if !self.peek_token_is(&tok) {
            self.peek_error(&tok);
            false
        } else {
            self.next_token();
            true
        }
    }

    fn peek_error(&mut self, tok: &Token) {
        let str = format!(
            "expected next token to be {:#?}, got {:#?} instead",
            tok, self.peek
        );
        self.errors.push(str);
    }

    fn peek_precedence(&self) -> Precedence {
        match &self.peek {
            Token::Eq => Precedence::Equals,
            Token::NotEq => Precedence::Equals,
            Token::Lt => Precedence::LessGreater,
            Token::Gt => Precedence::LessGreater,
            Token::Plus => Precedence::Sum,
            Token::Minus => Precedence::Sum,
            Token::Asterisk => Precedence::Product,
            Token::Slash => Precedence::Product,
            _ => Precedence::Lowest,
        }
    }

    fn cur_precedence(&self) -> Precedence {
        match &self.cur {
            Token::Eq => Precedence::Equals,
            Token::NotEq => Precedence::Equals,
            Token::Lt => Precedence::LessGreater,
            Token::Gt => Precedence::LessGreater,
            Token::Plus => Precedence::Sum,
            Token::Minus => Precedence::Sum,
            Token::Asterisk => Precedence::Product,
            Token::Slash => Precedence::Product,
            _ => Precedence::Lowest,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::{Expression, InfixOperator, Node, PrefixOperator, Statement};
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    struct BoolTest {
        input: &'static str,
        exp: bool,
    }

    struct PrefixIntTest {
        input: &'static str,
        oper: PrefixOperator,
        int_val: i64,
    }

    struct PrefixBoolTest {
        input: &'static str,
        oper: PrefixOperator,
        bool_val: bool,
    }

    struct InfixIntTest {
        input: &'static str,
        lval: i64,
        oper: InfixOperator,
        rval: i64,
    }

    struct InfixBoolTest {
        input: &'static str,
        lval: bool,
        oper: InfixOperator,
        rval: bool,
    }

    struct PrecedenceTest {
        input: &'static str,
        exp: &'static str,
    }

    fn test_let_statement(stmt: &Statement, name: &str) {
        if let Statement::LetStatement(ls) = stmt {
            let lit = ls.token_literal();
            assert_eq!(lit, "let");
            assert_eq!(ls.name.value.to_string(), name.to_owned());
            assert_eq!(ls.name.token_literal(), name.to_owned());
        } else {
            eprintln!("{:#?} is not a let statement", stmt);
            assert!(false);
        }
    }

    fn test_integer_exp(exp: &Expression, exp_int: i64) {
        if let Expression::Integer(il) = exp {
            assert_eq!(il.value, exp_int);
        } else {
            eprintln!("{:#?} is not an integer literal", exp);
            assert!(false);
        }
    }

    fn test_boolean_exp(exp: &Expression, exp_bool: bool) {
        if let Expression::Boolean(bl) = exp {
            assert_eq!(bl.value, exp_bool);
        } else {
            eprintln!("{:#?} is not a boolean literal", exp);
            assert!(false);
        }
    }

    fn check_errors(p: &Parser) {
        if p.errors_len() > 0 {
            for e in p.get_errors() {
                println!("{}", e);
            }
            panic!("parser had errors")
        }
    }

    #[test]
    fn test_let_statements() {
        let input = "let x = 5;
        let y = 10;
        let foobar = 838383;";
        let l = Lexer::new(&input);
        let mut p = Parser::new(l);
        let program = p.parse();
        check_errors(&p);
        let exps = vec!["x", "y", "foobar"];
        assert_eq!(program.statements.len(), 3);
        for (i, exp) in exps.iter().enumerate() {
            let stmt = &program.statements[i];
            test_let_statement(&stmt, exp);
        }
    }

    #[test]
    fn test_return_statements() {
        let input = "return 5;
        return 10;
        return 993322;";
        let l = Lexer::new(&input);
        let mut p = Parser::new(l);
        let program = p.parse();
        check_errors(&p);
        assert_eq!(program.statements.len(), 3);
        for stmt in program.statements.iter() {
            if let Statement::ReturnStatement(rs) = stmt {
                assert_eq!(rs.token_literal(), "return".to_string());
            } else {
                let s = format!("{:#?} is not a return statement", stmt);
                panic!("{}", s);
            }
        }
    }

    #[test]
    fn test_identifier_expression() {
        let input = "foobar";
        let l = Lexer::new(&input);
        let mut p = Parser::new(l);
        let program = p.parse();
        check_errors(&p);
        assert_eq!(program.statements.len(), 1);
        let stmt = &program.statements[0];
        if let Statement::ExpressionStatement(es) = stmt {
            if let Expression::Identifier(i) = &es.expression {
                assert_eq!(i.value.to_string(), "foobar".to_string());
            } else {
                let s = format!("{:#?} is not an identifier expression", es.expression);
                panic!("{}", s);
            }
        } else {
            let s = format!("{:#?} is not an expression statement", stmt);
            panic!("{}", s);
        }
    }

    #[test]
    fn test_integer_literal_expression() {
        let input = "5;";
        let l = Lexer::new(&input);
        let mut p = Parser::new(l);
        let program = p.parse();
        check_errors(&p);
        assert_eq!(program.statements.len(), 1);
        let stmt = &program.statements[0];
        if let Statement::ExpressionStatement(es) = stmt {
            if let Expression::Integer(il) = &es.expression {
                assert_eq!(il.value, 5);
            } else {
                let s = format!("{:#?} is not an integer literal expression", es.expression);
                panic!("{}", s);
            }
        } else {
            let s = format!("{:#?} is not an expression statement", stmt);
            panic!("{}", s);
        }
    }

    #[test]
    fn test_prefix_expressoins() {
        let prefix_int_tests = vec![
            PrefixIntTest {
                input: "!5;",
                oper: PrefixOperator::Bang,
                int_val: 5,
            },
            PrefixIntTest {
                input: "-15;",
                oper: PrefixOperator::Minus,
                int_val: 15,
            },
        ];
        let prefix_bool_tests = vec![
            PrefixBoolTest {
                input: "!true;",
                oper: PrefixOperator::Bang,
                bool_val: true,
            },
            PrefixBoolTest {
                input: "!false;",
                oper: PrefixOperator::Bang,
                bool_val: false,
            },
        ];
        for pt in prefix_int_tests.iter() {
            let l = Lexer::new(pt.input);
            let mut p = Parser::new(l);
            let program = p.parse();
            check_errors(&p);
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let Statement::ExpressionStatement(es) = stmt {
                if let Expression::PrefixExpression(pe) = &es.expression {
                    assert_eq!(pe.operator, pt.oper);
                    test_integer_exp(&pe.right, pt.int_val);
                } else {
                    let s = format!("{:#?} is not a prefix expression", es.expression);
                    panic!("{}", s);
                }
            } else {
                let s = format!("{:#?} is not an expression statement", stmt);
                panic!("{}", s);
            }
        }

        for pt in prefix_bool_tests.iter() {
            let l = Lexer::new(pt.input);
            let mut p = Parser::new(l);
            let program = p.parse();
            check_errors(&p);
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let Statement::ExpressionStatement(es) = stmt {
                if let Expression::PrefixExpression(pe) = &es.expression {
                    assert_eq!(pe.operator, pt.oper);
                    test_boolean_exp(&pe.right, pt.bool_val);
                } else {
                    let s = format!("{:#?} is not a prefix expression", es.expression);
                    panic!("{}", s);
                }
            } else {
                let s = format!("{:#?} is not an expression statement", stmt);
                panic!("{}", s);
            }
        }
    }

    #[test]
    fn test_infix_expressions() {
        let int_tests = vec![
            InfixIntTest {
                input: "5 + 5",
                lval: 5,
                oper: InfixOperator::Plus,
                rval: 5,
            },
            InfixIntTest {
                input: "5 - 5",
                lval: 5,
                oper: InfixOperator::Minus,
                rval: 5,
            },
            InfixIntTest {
                input: "5 * 5",
                lval: 5,
                oper: InfixOperator::Asterisk,
                rval: 5,
            },
            InfixIntTest {
                input: "5 / 5",
                lval: 5,
                oper: InfixOperator::Slash,
                rval: 5,
            },
            InfixIntTest {
                input: "5 > 5",
                lval: 5,
                oper: InfixOperator::Gt,
                rval: 5,
            },
            InfixIntTest {
                input: "5 < 5",
                lval: 5,
                oper: InfixOperator::Lt,
                rval: 5,
            },
            InfixIntTest {
                input: "5 == 5",
                lval: 5,
                oper: InfixOperator::Eq,
                rval: 5,
            },
            InfixIntTest {
                input: "5 != 5",
                lval: 5,
                oper: InfixOperator::NotEq,
                rval: 5,
            },
        ];

        let bool_tests = vec![
            InfixBoolTest {
                input: "true == true",
                lval: true,
                oper: InfixOperator::Eq,
                rval: true,
            },
            InfixBoolTest {
                input: "true != false",
                lval: true,
                oper: InfixOperator::NotEq,
                rval: false,
            },
            InfixBoolTest {
                input: "false == false",
                lval: false,
                oper: InfixOperator::Eq,
                rval: false,
            },
        ];

        for it in int_tests.iter() {
            let l = Lexer::new(it.input);
            let mut p = Parser::new(l);
            let program = p.parse();
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let Statement::ExpressionStatement(es) = stmt {
                if let Expression::InfixExpression(ie) = &es.expression {
                    assert_eq!(ie.operator, it.oper);
                    test_integer_exp(&ie.left, it.lval);
                    test_integer_exp(&ie.right, it.rval);
                } else {
                    let s = format!("{:#?} is not a prefix expression", es.expression);
                    panic!("{}", s);
                }
            } else {
                let s = format!("{:#?} is not an expression statement", stmt);
                panic!("{}", s);
            }
        }

        for it in bool_tests.iter() {
            let l = Lexer::new(it.input);
            let mut p = Parser::new(l);
            let program = p.parse();
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let Statement::ExpressionStatement(es) = stmt {
                if let Expression::InfixExpression(ie) = &es.expression {
                    assert_eq!(ie.operator, it.oper);
                    test_boolean_exp(&ie.left, it.lval);
                    test_boolean_exp(&ie.right, it.rval);
                } else {
                    let s = format!("{:#?} is not a prefix expression", es.expression);
                    panic!("{}", s);
                }
            } else {
                let s = format!("{:#?} is not an expression statement", stmt);
                panic!("{}", s);
            }
        }
    }

    #[test]
    fn operator_precedence() {
        let tests = vec![
            PrecedenceTest {
                input: "-a * b",
                exp: "((-a) * b)",
            },
            PrecedenceTest {
                input: "!-a",
                exp: "(!(-a))",
            },
            PrecedenceTest {
                input: "a + b + c",
                exp: "((a + b) + c)",
            },
            PrecedenceTest {
                input: "a + b - c",
                exp: "((a + b) - c)",
            },
            PrecedenceTest {
                input: "a * b * c",
                exp: "((a * b) * c)",
            },
            PrecedenceTest {
                input: "a * b / c",
                exp: "((a * b) / c)",
            },
            PrecedenceTest {
                input: "a + b / c",
                exp: "(a + (b / c))",
            },
            PrecedenceTest {
                input: "a + b * c + d / e - f",
                exp: "(((a + (b * c)) + (d / e)) - f)",
            },
            PrecedenceTest {
                input: "3 + 4; -5 * 5",
                exp: "(3 + 4)((-5) * 5)",
            },
            PrecedenceTest {
                input: "5 > 4 == 3 < 4",
                exp: "((5 > 4) == (3 < 4))",
            },
            PrecedenceTest {
                input: "5 < 4 != 3 > 4",
                exp: "((5 < 4) != (3 > 4))",
            },
            PrecedenceTest {
                input: "3 + 4 * 5 == 3 * 1 + 4 * 5",
                exp: "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))",
            },
            PrecedenceTest {
                input: "3 + 4 * 5 == 3 * 1 + 4 * 5",
                exp: "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))",
            },
            PrecedenceTest {
                input: "true",
                exp: "true",
            },
            PrecedenceTest {
                input: "false",
                exp: "false",
            },
            PrecedenceTest {
                input: "3 > 5 == false",
                exp: "((3 > 5) == false)",
            },
            PrecedenceTest {
                input: "3 < 5 == true",
                exp: "((3 < 5) == true)",
            },
            PrecedenceTest {
                input: "1 + (2 + 3) + 4",
                exp: "((1 + (2 + 3)) + 4)",
            },
            PrecedenceTest {
                input: "(5 + 5) * 2",
                exp: "((5 + 5) * 2)",
            },
            PrecedenceTest {
                input: "2 / (5 + 5)",
                exp: "(2 / (5 + 5))",
            },
            PrecedenceTest {
                input: "-(5 + 5)",
                exp: "(-(5 + 5))",
            },
            PrecedenceTest {
                input: "!(true == true)",
                exp: "(!(true == true))",
            },
        ];

        for t in tests.iter() {
            let l = Lexer::new(t.input);
            let mut p = Parser::new(l);
            let program = p.parse();
            let s = program.string();
            assert_eq!(s, t.exp);
        }
    }

    #[test]
    fn test_boolean_literal() {
        let tests = vec![
            BoolTest {
                input: "true",
                exp: true,
            },
            BoolTest {
                input: "false",
                exp: false,
            },
        ];
        for t in tests.iter() {
            let l = Lexer::new(t.input);
            let mut p = Parser::new(l);
            let program = p.parse();
            check_errors(&p);
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let Statement::ExpressionStatement(es) = stmt {
                test_boolean_exp(&es.expression, t.exp);
            } else {
                let s = format!("{:#?} is not an expression statement", stmt);
                panic!("{}", s);
            }
        }
    }
}
