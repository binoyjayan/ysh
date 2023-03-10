use std::cell::RefCell;
use std::rc::Rc;

use super::environment::*;
use super::error::RTError;
use super::object::*;
use crate::parser::ast::expr::*;
use crate::parser::ast::stmt::BlockStatement;
use crate::parser::ast::stmt::Statement;
use crate::parser::ast::*;
use crate::token::*;

pub struct Evaluator {}

impl Evaluator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn eval_program(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        program: Program,
    ) -> Result<Object, RTError> {
        self.eval_statements(env, program.statements)
    }

    // While evaluating block statements, do not unwrap return value.
    // Only check if it is a return value and if so, return the
    // Object::Return(val) object. This is so that a nested block
    // statement can return value correctly. This helps in the outer
    // block also return the wrapped return i.e. Object::Return(val)
    // Unwrapping only happens while executing the outer most block
    // statement which is a statement one level down the program.
    fn eval_statements_nounwrap(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        statements: Vec<Statement>,
    ) -> Result<Object, RTError> {
        let mut result = Object::Nil;
        for stmt in statements {
            result = self.eval_statement(env, stmt)?;
            if let Object::Return(_) = result {
                return Ok(result);
            }
        }
        Ok(result)
    }

    fn eval_block_statement(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        stmt: BlockStatement,
    ) -> Result<Object, RTError> {
        self.eval_statements_nounwrap(env, stmt.statements)
    }

    // Unwrap return values here since this is the outer most block
    fn eval_statements(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        statements: Vec<Statement>,
    ) -> Result<Object, RTError> {
        let result = self.eval_statements_nounwrap(env, statements)?;
        if let Object::Return(retval) = result {
            return Ok(*retval);
        }
        Ok(result)
    }

    // Wrap the return value in a Return object
    fn eval_return_stmt(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        expr: Expression,
    ) -> Result<Object, RTError> {
        let value = self.eval_expression(env, expr)?;
        Ok(Object::Return(Box::new(value)))
    }

    fn eval_expression(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        expr: Expression,
    ) -> Result<Object, RTError> {
        match expr {
            Expression::Number(num) => Ok(Object::Number(num.value)),
            Expression::Bool(num) => Ok(Object::Bool(num.value)),
            Expression::Unary(unary) => {
                let right = self.eval_expression(env, *unary.right)?;
                self.eval_prefix_expr(&unary.operator, right, unary.token.line)
            }
            Expression::Binary(binary) => {
                let left = self.eval_expression(env, *binary.left)?;
                let right = self.eval_expression(env, *binary.right)?;
                self.eval_infix_expr(&binary.operator, left, right, binary.token.line)
            }
            Expression::If(expr) => {
                let condition = self.eval_expression(env, *expr.condition)?;
                if Self::is_truthy(condition) {
                    return self.eval_block_statement(env, expr.then_stmt);
                } else {
                    if let Some(else_stmt) = expr.else_stmt {
                        return self.eval_block_statement(env, else_stmt);
                    }
                }
                // if the condition is false, the expressions that do not have
                // an else evaluates to a nil object
                Ok(Object::Nil)
            }
            Expression::Function(expr) => Ok(self.eval_function_expr(env, expr)),
            Expression::Ident(expr) => self.eval_identifier_expr(env, &expr.token),
            Expression::Call(expr) => Ok(self.eval_call_expr(env, expr)?),
            _ => Ok(Object::Nil),
        }
    }

    // Evaluate a vector of expressions, typically arguments to a function call
    // Note that the arguments are evaluated from left to right.
    fn eval_expressions(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        exprs: Vec<Expression>,
    ) -> Result<Vec<Object>, RTError> {
        let mut result = Vec::new();
        for expr in exprs {
            let obj = self.eval_expression(env, expr)?;
            result.push(obj);
        }
        Ok(result)
    }

    fn eval_let_stmt(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        name: &Identifier,
        expr: Expression,
    ) -> Result<Object, RTError> {
        let value = self.eval_expression(env, expr)?;
        let name = name.token.clone();
        env.borrow_mut().set(&name, value);
        Ok(Object::Nil)
    }

    fn eval_statement(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        stmt: Statement,
    ) -> Result<Object, RTError> {
        match stmt {
            Statement::Expr(stmt) => self.eval_expression(env, stmt.value),
            Statement::Return(stmt) => self.eval_return_stmt(env, stmt.value),
            Statement::Let(stmt) => self.eval_let_stmt(env, &stmt.name, stmt.value),
            _ => Ok(Object::Nil),
        }
    }

    fn is_truthy(obj: Object) -> bool {
        match obj {
            Object::Nil => false,
            Object::Bool(b) => b,
            Object::Number(n) => n != 0.,
            _ => true,
        }
    }

    fn eval_prefix_expr(
        &self,
        operator: &str,
        right: Object,
        line: usize,
    ) -> Result<Object, RTError> {
        match operator {
            "!" => Ok(self.eval_bang_operator_expr(right)),
            "-" => self.eval_minus_operator_expr(right, line),
            _ => Err(RTError::new("invalid prefix operator", line)),
        }
    }

    // Does not return runtime error
    fn eval_bang_operator_expr(&self, right: Object) -> Object {
        Object::Bool(right.is_falsey())
    }

    fn eval_minus_operator_expr(&self, right: Object, line: usize) -> Result<Object, RTError> {
        match right {
            Object::Number(num) => Ok(Object::Number(-num)),
            _ => Err(RTError::new("invalid unary operation", line)),
        }
    }

    fn eval_infix_expr(
        &self,
        operator: &str,
        left: Object,
        right: Object,
        line: usize,
    ) -> Result<Object, RTError> {
        match (left, right) {
            (Object::Number(left), Object::Number(right)) => match operator {
                "+" => Ok(Object::Number(left + right)),
                "-" => Ok(Object::Number(left - right)),
                "*" => Ok(Object::Number(left * right)),
                "/" => Ok(Object::Number(left / right)),
                "<" => Ok(Object::Bool(left < right)),
                ">" => Ok(Object::Bool(left > right)),
                "==" => Ok(Object::Bool(left == right)),
                "!=" => Ok(Object::Bool(left != right)),
                _ => Err(RTError::new("invalid binary operator", line)),
            },
            (Object::Bool(left), Object::Bool(right)) => match operator {
                "==" => Ok(Object::Bool(left == right)),
                "!=" => Ok(Object::Bool(left != right)),
                _ => Err(RTError::new("invalid binary operation", line)),
            },
            _ => Err(RTError::new("invalid binary operation", line)),
        }
    }

    fn eval_identifier_expr(
        &self,
        environment: &Rc<RefCell<Environment>>,
        token: &Token,
    ) -> Result<Object, RTError> {
        if let Some(obj) = environment.borrow().get(&token.literal.clone()) {
            Ok(obj)
        } else {
            Err(RTError::new(
                &format!("Undefined identifier: '{}'", token.literal),
                token.line,
            ))
        }
    }

    // Evaluate expression that defines a function
    fn eval_function_expr(
        &self,
        environment: &Rc<RefCell<Environment>>,
        func: FunctionLiteral,
    ) -> Object {
        Object::Func(Function {
            params: func.params,
            body: func.body,
            env: environment.clone(),
        })
    }

    // Evaluate call expression (e.g. function calls)
    // First use 'eval_expression' to get the function that needs to be called.
    // It can be an 'Identifier' or a 'FunctionLiteral'. It evaluates to a
    // 'Function' object. To call the function, first evaluate the list of
    // arguments which is evaluating a list of expressions.
    fn eval_call_expr(
        &mut self,
        env: &Rc<RefCell<Environment>>,
        call: CallExpr,
    ) -> Result<Object, RTError> {
        let function = self.eval_expression(env, *call.func)?;
        let args = self.eval_expressions(env, (*call.args).to_vec())?;
        if let Object::Func(func) = function {
            self.invoke_function_call(&func, args)
        } else {
            Err(RTError::new(
                &format!("Not a function: '{}'", call.token.literal),
                call.token.line,
            ))
        }
    }

    // This function creates a new function environment that is enclosed by
    // the function's environment. In this new enclosed environment, it binds
    /// the argument of the function calls to the function's parameter names.
    fn invoke_function_call(
        &mut self,
        function: &Function,
        args: Vec<Object>,
    ) -> Result<Object, RTError> {
        // Create extended env
        let mut extended_env = Environment::new_enclosing(function.env.clone());
        // Convert arguments to params
        for (i, param) in function.params.iter().enumerate() {
            extended_env.set(&param.token, args[i].clone())
        }
        // TODO: Do not clone the block statements
        self.eval_statements(
            &mut Rc::new(RefCell::new(extended_env)),
            function.body.statements,
        )
    }
}
