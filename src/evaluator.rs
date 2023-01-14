use crate::object::*;
use crate::parser::ast::expr::*;
use crate::parser::ast::stmt::Statement;
use crate::parser::ast::*;
use crate::parser::*;
use crate::scanner::*;

pub fn eval_program(program: Program) -> Object {
    eval_statements(program.statements)
}

fn eval_statements(statements: Vec<Statement>) -> Object {
    let mut result = Object::Nil;
    for stmt in statements {
        result = eval_statement(stmt)
    }
    result
}

fn eval_expression(expr: Expression) -> Object {
    match expr {
        Expression::Number(num) => Object::Number(num.value),
        Expression::Bool(num) => Object::Bool(num.value),
        Expression::Unary(unary) => {
            let right = eval_expression(*unary.right);
            eval_prefix_expr(&unary.operator, right)
        }
        _ => Object::Nil,
    }
}

fn eval_statement(stmt: Statement) -> Object {
    match stmt {
        Statement::Expr(stmt) => eval_expression(stmt.value),
        _ => Object::Nil,
    }
}

fn eval_prefix_expr(operator: &str, right: Object) -> Object {
    match operator {
        "!" => eval_bang_operator_expr(right),
        "-" => eval_minus_operator_expr(right),
        _ => Object::Nil,
    }
}

fn eval_bang_operator_expr(right: Object) -> Object {
    match right {
        Object::Bool(true) => Object::Bool(false),
        Object::Bool(false) => Object::Bool(true),
        Object::Nil => Object::Bool(true),
        _ => Object::Bool(false),
    }
}

fn eval_minus_operator_expr(right: Object) -> Object {
    match right {
        Object::Number(num) => Object::Number(-num),
        _ => Object::Nil,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn check_parse_errors(parser: &Parser) {
        let errors = parser.parse_errors();
        if errors.is_empty() {
            return;
        }
        for msg in errors {
            eprintln!("{}", msg);
        }
        panic!("{} parse error(s)", errors.len());
    }

    fn test_numeric_object(evaluated: Object, expected: f64) {
        if let Object::Number(num) = evaluated {
            assert_eq!(
                num, expected,
                "object has wrong value. got={}, want={}",
                num, expected
            );
        } else {
            panic!("object is not numeric. got={}", evaluated);
        }
    }

    fn test_boolean_object(evaluated: Object, expected: bool) {
        if let Object::Bool(b) = evaluated {
            assert_eq!(
                b, expected,
                "object has wrong value. got={}, want={}",
                b, expected
            );
        } else {
            panic!("object is not boolean. got={}", evaluated);
        }
    }

    fn test_eval(input: &str) -> Object {
        let scanner = Scanner::new(input);
        let mut parser = Parser::new(scanner);
        let program = parser.parse_program();
        check_parse_errors(&parser);
        eval_program(program)
    }

    #[test]
    fn test_eval_numeric_expr() {
        struct NumericObj {
            input: &'static str,
            expected: f64,
        }
        let numeric_tests = vec![
            NumericObj {
                input: "5",
                expected: 5.,
            },
            NumericObj {
                input: "10",
                expected: 10.,
            },
            NumericObj {
                input: "-5",
                expected: -5.,
            },
            NumericObj {
                input: "-10",
                expected: -10.,
            },
        ];
        for test in numeric_tests {
            let evaluated = test_eval(test.input);
            test_numeric_object(evaluated, test.expected);
        }
    }

    #[test]
    fn test_eval_boolean_expr() {
        struct BooleanObj {
            input: &'static str,
            expected: bool,
        }
        let boolean_tests = vec![
            BooleanObj {
                input: "true",
                expected: true,
            },
            BooleanObj {
                input: "false",
                expected: false,
            },
        ];
        for test in boolean_tests {
            let evaluated = test_eval(test.input);
            test_boolean_object(evaluated, test.expected);
        }
    }

    #[test]
    fn test_eval_bang_operator() {
        struct BangExpr {
            input: &'static str,
            expected: bool,
        }
        let boolean_tests = vec![
            BangExpr {
                input: "!true",
                expected: false,
            },
            BangExpr {
                input: "!false",
                expected: true,
            },
            BangExpr {
                input: "!5",
                expected: false,
            },
            BangExpr {
                input: "!!true",
                expected: true,
            },
            BangExpr {
                input: "!!false",
                expected: false,
            },
        ];
        for test in boolean_tests {
            let evaluated = test_eval(test.input);
            test_boolean_object(evaluated, test.expected);
        }
    }
}
