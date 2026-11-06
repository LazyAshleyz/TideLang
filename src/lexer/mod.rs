#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    And,
    Or,
    Not,
    True,
    False,
    Import,
    // Types
    Int,
    Bn,
    Ft,
    StrType,
    Array,
    // Literals and names
    Ident(String),
    Number(i64),
    Float(f64),
    String(String),
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    EqEq,
    NotEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Dot,
    // End Of File
    Eof,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let done = token == Token::Eof;
            tokens.push(token);
            if done {
                break;
            }
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
            }
        }
        c
    }

    // Consumes the next character only if it matches 'expected'
    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                // '//' comments run to the end of the line
                Some('/') if self.peek_next() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace_and_comments();

        let c = match self.advance() {
            Some(c) => c,
            None => return Ok(Token::Eof),
        };

        let token = match c {
            '+' => {
                if self.match_char('=') { Token::PlusAssign } else { Token::Plus }
            }
            '-' => {
                if self.match_char('=') { Token::MinusAssign } else { Token::Minus }
            }
            '*' => {
                if self.match_char('=') { Token::StarAssign } else { Token::Star }
            }
            '/' => {
                if self.match_char('=') { Token::SlashAssign } else { Token::Slash }
            }
            '%' => {
                if self.match_char('=') { Token::PercentAssign } else { Token::Percent }
            }
            '=' => {
                if self.match_char('=') { Token::EqEq } else { Token::Assign }
            }
            '<' => {
                if self.match_char('=') { Token::LessEq } else { Token::Less }
            }
            '>' => {
                if self.match_char('=') { Token::GreaterEq } else { Token::Greater }
            }
            '!' => {
                if self.match_char('=') {
                    Token::NotEq
                } else {
                    return Err(format!(
                        "line {}: expected '=' after '!' (use 'not' for negation)",
                        self.line
                    ));
                }
            }
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            '.' => Token::Dot,
            '"' => self.string()?,
            c if c.is_ascii_digit() => self.number(c)?,
            c if c.is_alphabetic() || c == '_' => self.identifier(c),
            other => {
                return Err(format!("line {}: unexpected character '{}'", self.line, other));
            }
        };

        Ok(token)
    }

    fn string(&mut self) -> Result<Token, String> {
        let start_line = self.line;
        let mut text = String::new();
        loop {
            match self.advance() {
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => text.push('\n'),
                    Some('t') => text.push('\t'),
                    Some('"') => text.push('"'),
                    Some('\\') => text.push('\\'),
                    Some(other) => {
                        return Err(format!(
                            "line {}: unknown escape '\\{}'",
                            self.line, other
                        ));
                    }
                    None => {
                        return Err(format!("line {}: unterminated string", start_line));
                    }
                },
                Some(c) => text.push(c),
                None => {
                    return Err(format!("line {}: unterminated string", start_line));
                }
            }
        }
        Ok(Token::String(text))
    }

    fn number(&mut self, first: char) -> Result<Token, String> {
        let mut text = String::new();
        text.push(first);
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // A '.' followed by a digit makes it a float, like 3.14
        if self.peek() == Some('.') && self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
            text.push('.');
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    text.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            return match text.parse::<f64>() {
                Ok(f) => Ok(Token::Float(f)),
                Err(_) => Err(format!("line {}: invalid number '{}'", self.line, text)),
            };
        }

        match text.parse::<i64>() {
            Ok(n) => Ok(Token::Number(n)),
            Err(_) => Err(format!("line {}: number '{}' is too large", self.line, text)),
        }
    }

    fn identifier(&mut self, first: char) -> Token {
        let mut text = String::new();
        text.push(first);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                text.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match text.as_str() {
            "fn" => Token::Fn,
            "return" => Token::Return,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "in" => Token::In,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            "true" => Token::True,
            "false" => Token::False,
            "import" => Token::Import,
            "int" => Token::Int,
            "bn" => Token::Bn,
            "ft" => Token::Ft,
            "str" => Token::StrType,
            "array" => Token::Array,
            _ => Token::Ident(text),
        }
    }
}