use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    // A whole file is a list of statements
    pub fn parse(&mut self) -> Result<Program, String> {
        let mut program = Vec::new();
        while !self.is_at_end() {
            program.push(self.statement()?);
        }
        Ok(program)
    }

    // ---------- helpers ----------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn is_at_end(&self) -> bool {
        *self.peek() == Token::Eof
    }

    // Returns the current token and moves forward (never moves past Eof)
    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if !self.is_at_end() {
            self.pos += 1;
        }
        token
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek() == expected
    }

    // Consumes the token only if it matches
    fn match_token(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    // Consumes the token or errors
    fn expect(&mut self, expected: Token, what: &str) -> Result<(), String> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected {}, found {:?}", what, self.peek()))
        }
    }

    fn expect_ident(&mut self, what: &str) -> Result<String, String> {
        match self.advance() {
            Token::Ident(name) => Ok(name),
            other => Err(format!("expected {}, found {:?}", what, other)),
        }
    }

    fn is_type_token(&self) -> bool {
        matches!(
            self.peek(),
            Token::Int | Token::Bn | Token::Ft | Token::StrType | Token::Array
        )
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match self.advance() {
            Token::Int => Ok(Type::Int),
            Token::Bn => Ok(Type::Bn),
            Token::Ft => Ok(Type::Ft),
            Token::StrType => Ok(Type::Str),
            Token::Array => Ok(Type::Array),
            other => Err(format!("expected a type, found {:?}", other)),
        }
    }

    // Reads =, +=, -=, *=, /= or %=
    fn assign_op(&mut self) -> Result<AssignOp, String> {
        match self.advance() {
            Token::Assign => Ok(AssignOp::Assign),
            Token::PlusAssign => Ok(AssignOp::AddAssign),
            Token::MinusAssign => Ok(AssignOp::SubAssign),
            Token::StarAssign => Ok(AssignOp::MulAssign),
            Token::SlashAssign => Ok(AssignOp::DivAssign),
            Token::PercentAssign => Ok(AssignOp::ModAssign),
            other => Err(format!("expected an assignment operator, found {:?}", other)),
        }
    }

    // `x = 1`, `x += 1`, `nums[0] = 5` with no trailing semicolon
    // (used inside `for (...)` headers)
    fn assignment_no_semi(&mut self) -> Result<Stmt, String> {
        let name = self.expect_ident("variable name")?;
        if self.match_token(&Token::LBracket) {
            let index = self.expression()?;
            self.expect(Token::RBracket, "']' after index")?;
            let op = self.assign_op()?;
            let value = self.expression()?;
            return Ok(Stmt::IndexAssign { name, index, op, value });
        }
        let op = self.assign_op()?;
        let value = self.expression()?;
        Ok(Stmt::Assign { name, op, value })
    }

    // ---------- statements ----------

    fn statement(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            Token::Import => self.import_statement(),
            Token::Fn => self.function(),
            Token::If => self.if_statement(),
            Token::While => self.while_statement(),
            Token::For => self.for_statement(),
            Token::Return => self.return_statement(),
            Token::Break => {
                self.advance();
                self.expect(Token::Semicolon, "';' after 'break'")?;
                Ok(Stmt::Break)
            }
            Token::Continue => {
                self.advance();
                self.expect(Token::Semicolon, "';' after 'continue'")?;
                Ok(Stmt::Continue)
            }
            Token::Int | Token::Bn | Token::Ft | Token::StrType | Token::Array => {
                self.var_decl()
            }
            Token::Ident(_) => self.ident_statement(),
            other => Err(format!("unexpected token {:?} at start of statement", other)),
        }
    }

    // { statement* }
    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::LBrace, "'{'")?;
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            stmts.push(self.statement()?);
        }
        self.expect(Token::RBrace, "'}'")?;
        Ok(stmts)
    }

    // import std;
    fn import_statement(&mut self) -> Result<Stmt, String> {
        self.advance(); // import
        let module = self.expect_ident("module name after 'import'")?;
        self.expect(Token::Semicolon, "';' after import")?;
        Ok(Stmt::Import(module))
    }

    // fn name(int a, int b) { ... }
    fn function(&mut self) -> Result<Stmt, String> {
        self.advance(); // fn
        let name = self.expect_ident("function name")?;
        self.expect(Token::LParen, "'(' after function name")?;

        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                let ty = self.parse_type()?;
                let param_name = self.expect_ident("parameter name")?;
                params.push(Param { ty, name: param_name });
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(Token::RParen, "')' after parameters")?;

        let body = self.block()?;
        Ok(Stmt::Function(FunctionDecl { name, params, body }))
    }

    // if (cond) { ... } else { ... }   (also supports: else if)
    fn if_statement(&mut self) -> Result<Stmt, String> {
        self.advance(); // if
        self.expect(Token::LParen, "'(' after 'if'")?;
        let condition = self.expression()?;
        self.expect(Token::RParen, "')' after condition")?;
        let then_block = self.block()?;

        let else_block = if self.match_token(&Token::Else) {
            if self.check(&Token::If) {
                Some(vec![self.if_statement()?])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };

        Ok(Stmt::If { condition, then_block, else_block })
    }

    // while (cond) { ... }
    fn while_statement(&mut self) -> Result<Stmt, String> {
        self.advance(); // while
        self.expect(Token::LParen, "'(' after 'while'")?;
        let condition = self.expression()?;
        self.expect(Token::RParen, "')' after condition")?;
        let body = self.block()?;
        Ok(Stmt::While { condition, body })
    }

    // for (int i = 0; i < 10; i += 1) { ... }
    // for (int x in collection) { ... }
    fn for_statement(&mut self) -> Result<Stmt, String> {
        self.advance(); // for
        self.expect(Token::LParen, "'(' after 'for'")?;

        // for-in looks like: a type, a name, then `in`
        let looks_like_for_in = self.is_type_token()
            && matches!(self.tokens.get(self.pos + 1), Some(Token::Ident(_)))
            && self.tokens.get(self.pos + 2) == Some(&Token::In);

        if looks_like_for_in {
            let ty = self.parse_type()?;
            let var = self.expect_ident("loop variable name")?;
            self.expect(Token::In, "'in'")?;
            let iterable = self.expression()?;
            self.expect(Token::RParen, "')' after loop header")?;
            let body = self.block()?;
            return Ok(Stmt::ForIn { ty, var, iterable, body });
        }

        // C-style: init ; condition ; step
        let init = if self.match_token(&Token::Semicolon) {
            None
        } else if self.is_type_token() {
            // var_decl consumes its own ';'
            Some(Box::new(self.var_decl()?))
        } else {
            let stmt = self.assignment_no_semi()?;
            self.expect(Token::Semicolon, "';' after loop initializer")?;
            Some(Box::new(stmt))
        };

        let condition = if self.check(&Token::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(Token::Semicolon, "';' after loop condition")?;

        let step = if self.check(&Token::RParen) {
            None
        } else {
            Some(Box::new(self.assignment_no_semi()?))
        };
        self.expect(Token::RParen, "')' after loop header")?;

        let body = self.block()?;
        Ok(Stmt::For { init, condition, step, body })
    }

    // return expr;   or just   return;
    fn return_statement(&mut self) -> Result<Stmt, String> {
        self.advance(); // return
        if self.match_token(&Token::Semicolon) {
            return Ok(Stmt::Return(None));
        }
        let value = self.expression()?;
        self.expect(Token::Semicolon, "';' after return value")?;
        Ok(Stmt::Return(Some(value)))
    }

    // int x = expr;
    fn var_decl(&mut self) -> Result<Stmt, String> {
        let ty = self.parse_type()?;
        let name = self.expect_ident("variable name")?;
        self.expect(Token::Assign, "'=' after variable name")?;
        let value = self.expression()?;
        self.expect(Token::Semicolon, "';' after declaration")?;
        Ok(Stmt::VarDecl { ty, name, value })
    }

    // Statements that start with a name:
    //   x = 1;   x += 1;   nums[0] = 5;   greet();   std.push(nums, 4);
    fn ident_statement(&mut self) -> Result<Stmt, String> {
        let name = self.expect_ident("name")?;

        match self.peek().clone() {
            Token::LBracket => {
                self.advance(); // [
                let index = self.expression()?;
                self.expect(Token::RBracket, "']' after index")?;
                let op = self.assign_op()?;
                let value = self.expression()?;
                self.expect(Token::Semicolon, "';' after assignment")?;
                Ok(Stmt::IndexAssign { name, index, op, value })
            }
            Token::Dot => {
                self.advance(); // .
                let func = self.expect_ident("function name after '.'")?;
                let call = self.finish_call(Some(name), func)?;
                self.expect(Token::Semicolon, "';' after call")?;
                Ok(Stmt::ExprStmt(call))
            }
            Token::LParen => {
                let call = self.finish_call(None, name)?;
                self.expect(Token::Semicolon, "';' after call")?;
                Ok(Stmt::ExprStmt(call))
            }
            Token::Assign
            | Token::PlusAssign
            | Token::MinusAssign
            | Token::StarAssign
            | Token::SlashAssign
            | Token::PercentAssign => {
                let op = self.assign_op()?;
                let value = self.expression()?;
                self.expect(Token::Semicolon, "';' after assignment")?;
                Ok(Stmt::Assign { name, op, value })
            }
            other => Err(format!("unexpected {:?} after '{}'", other, name)),
        }
    }

    // ---------- expressions ----------
    // Lowest precedence first:
    //   or -> and -> not -> == != -> < > <= >= -> + - -> * / % -> unary - -> [index] -> basics

    fn expression(&mut self) -> Result<Expr, String> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.and_expr()?;
        while self.match_token(&Token::Or) {
            let right = self.and_expr()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.not_expr()?;
        while self.match_token(&Token::And) {
            let right = self.not_expr()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // `not` sits below comparisons, so `not a == b` means `not (a == b)`
    fn not_expr(&mut self) -> Result<Expr, String> {
        if self.match_token(&Token::Not) {
            let operand = self.not_expr()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            });
        }
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr, String> {
        let mut left = self.comparison()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinaryOp::Eq,
                Token::NotEq => BinaryOp::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.comparison()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.additive()?;
        loop {
            let op = match self.peek() {
                Token::Less => BinaryOp::Less,
                Token::Greater => BinaryOp::Greater,
                Token::LessEq => BinaryOp::LessEq,
                Token::GreaterEq => BinaryOp::GreaterEq,
                _ => break,
            };
            self.advance();
            let right = self.additive()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let mut left = self.multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.multiplicative()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // -x
    fn unary(&mut self) -> Result<Expr, String> {
        if self.match_token(&Token::Minus) {
            let operand = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
            });
        }
        self.postfix()
    }

    // something[index][index]...
    fn postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.primary()?;
        while self.match_token(&Token::LBracket) {
            let index = self.expression()?;
            self.expect(Token::RBracket, "']' after index")?;
            expr = Expr::Index {
                target: Box::new(expr),
                index: Box::new(index),
            };
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Token::Number(n) => Ok(Expr::Number(n)),
            Token::Float(f) => Ok(Expr::Float(f)),
            Token::String(s) => Ok(Expr::String(s)),
            Token::True => Ok(Expr::Bool(true)),
            Token::False => Ok(Expr::Bool(false)),
            Token::Ident(name) => {
                if self.check(&Token::LParen) {
                    self.finish_call(None, name)
                } else if self.match_token(&Token::Dot) {
                    let func = self.expect_ident("function name after '.'")?;
                    self.finish_call(Some(name), func)
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            // ( expression )
            Token::LParen => {
                let inner = self.expression()?;
                self.expect(Token::RParen, "')'")?;
                Ok(inner)
            }
            // [a, b, c]
            Token::LBracket => {
                let mut items = Vec::new();
                if !self.check(&Token::RBracket) {
                    loop {
                        items.push(self.expression()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.expect(Token::RBracket, "']'")?;
                Ok(Expr::Array(items))
            }
            other => Err(format!("unexpected token {:?} in expression", other)),
        }
    }

    // Parses `(arg, arg, ...)` after a function name
    fn finish_call(&mut self, module: Option<String>, name: String) -> Result<Expr, String> {
        self.expect(Token::LParen, "'('")?;
        let mut args = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                args.push(self.expression()?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(Token::RParen, "')' after arguments")?;
        Ok(Expr::Call { module, name, args })
    }
}