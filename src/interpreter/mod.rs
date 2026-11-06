use crate::ast::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::rc::Rc;

// What values look like while the program runs.
// Arrays are shared references: `array b = a;` makes both names point at the
// same array, so std.push(a, 1) is visible through b too (same as Python).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Rc<RefCell<Vec<Value>>>),
    Null, // what a function gives back when it doesn't return anything
}

fn new_array(items: Vec<Value>) -> Value {
    Value::Array(Rc::new(RefCell::new(items)))
}

// 3.0 prints as "3.0", not "3"
fn format_float(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 {
        format!("{:.1}", f)
    } else {
        format!("{}", f)
    }
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "ft",
            Value::Bool(_) => "bn",
            Value::Str(_) => "str",
            Value::Array(_) => "array",
            Value::Null => "nothing",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(x) => write!(f, "{}", format_float(*x)),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Str(s) => write!(f, "{}", s),
            Value::Array(items) => {
                let items = items.borrow();
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match item {
                        Value::Str(s) => write!(f, "\"{}\"", s)?,
                        other => write!(f, "{}", other)?,
                    }
                }
                write!(f, "]")
            }
            Value::Null => write!(f, "null"),
        }
    }
}

// How a statement finished: normally, or by return / break / continue
enum Flow {
    Normal,
    Return(Value),
    Break,
    Continue,
}

pub struct Interpreter {
    functions: HashMap<String, FunctionDecl>,
    imports: HashSet<String>,
    script_args: Vec<String>,
    // One frame per function call, each frame is a stack of block scopes.
    // frames[0][0] is the global scope.
    frames: Vec<Vec<HashMap<String, Value>>>,
}

impl Interpreter {
    pub fn new(script_args: Vec<String>) -> Self {
        Interpreter {
            functions: HashMap::new(),
            imports: HashSet::new(),
            script_args,
            frames: vec![vec![HashMap::new()]],
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), String> {
        // Register top-level functions first so they can be called before
        // they appear in the file
        for stmt in program {
            if let Stmt::Function(f) = stmt {
                self.functions.insert(f.name.clone(), f.clone());
            }
        }

        match self.exec_stmts(program)? {
            Flow::Normal => Ok(()),
            Flow::Return(_) => Err("'return' used outside of a function".to_string()),
            Flow::Break => Err("'break' used outside of a loop".to_string()),
            Flow::Continue => Err("'continue' used outside of a loop".to_string()),
        }
    }

    // ---------- variables ----------

    // Finds which (frame, scope) holds a variable: the innermost scope of the
    // current function first, then the globals
    fn locate(&self, name: &str) -> Option<(usize, usize)> {
        let current = self.frames.len() - 1;
        for scope_idx in (0..self.frames[current].len()).rev() {
            if self.frames[current][scope_idx].contains_key(name) {
                return Some((current, scope_idx));
            }
        }
        if current != 0 && self.frames[0][0].contains_key(name) {
            return Some((0, 0));
        }
        None
    }

    fn get_var(&self, name: &str) -> Result<Value, String> {
        match self.locate(name) {
            Some((f, s)) => Ok(self.frames[f][s].get(name).unwrap().clone()),
            None => Err(format!("undefined variable '{}'", name)),
        }
    }

    fn set_var(&mut self, name: &str, value: Value) -> Result<(), String> {
        match self.locate(name) {
            Some((f, s)) => {
                self.frames[f][s].insert(name.to_string(), value);
                Ok(())
            }
            None => Err(format!("undefined variable '{}'", name)),
        }
    }

    // ---------- statements ----------

    fn exec_stmts(&mut self, stmts: &[Stmt]) -> Result<Flow, String> {
        for stmt in stmts {
            let flow = self.exec_stmt(stmt)?;
            if !matches!(flow, Flow::Normal) {
                return Ok(flow);
            }
        }
        Ok(Flow::Normal)
    }

    // Runs statements inside a fresh scope, so variables made in a { } block
    // disappear when it ends
    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Flow, String> {
        self.frames.last_mut().unwrap().push(HashMap::new());
        let result = self.exec_stmts(stmts);
        self.frames.last_mut().unwrap().pop();
        result
    }

    // The loop part of a C-style for. The caller wraps it in a scope so that
    // a variable declared in the header disappears afterwards.
    fn run_for(
        &mut self,
        init: &Option<Box<Stmt>>,
        condition: &Option<Expr>,
        step: &Option<Box<Stmt>>,
        body: &[Stmt],
    ) -> Result<Flow, String> {
        if let Some(init_stmt) = init {
            self.exec_stmt(init_stmt)?;
        }
        loop {
            if let Some(cond) = condition {
                let value = self.eval_expr(cond)?;
                if !is_truthy(&value) {
                    break;
                }
            }
            match self.exec_block(body)? {
                Flow::Normal | Flow::Continue => {}
                Flow::Break => break,
                Flow::Return(value) => return Ok(Flow::Return(value)),
            }
            if let Some(step_stmt) = step {
                self.exec_stmt(step_stmt)?;
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow, String> {
        match stmt {
            Stmt::Import(module) => {
                if module != "std" {
                    return Err(format!(
                        "unknown module '{}' (available modules: std)",
                        module
                    ));
                }
                self.imports.insert(module.clone());
                Ok(Flow::Normal)
            }

            Stmt::VarDecl { ty, name, value } => {
                let val = self.eval_expr(value)?;
                let val = coerce_to_type(ty, val, name)?;
                let scope = self.frames.last_mut().unwrap().last_mut().unwrap();
                if scope.contains_key(name) {
                    return Err(format!("variable '{}' is already declared", name));
                }
                scope.insert(name.clone(), val);
                Ok(Flow::Normal)
            }

            Stmt::Assign { name, op, value } => {
                let rhs = self.eval_expr(value)?;
                let new_value = match assign_to_binary(op) {
                    None => rhs,
                    Some(bin) => {
                        let current = self.get_var(name)?;
                        binary_op(&bin, current, rhs)?
                    }
                };
                self.set_var(name, new_value)?;
                Ok(Flow::Normal)
            }

            Stmt::IndexAssign { name, index, op, value } => {
                let rhs = self.eval_expr(value)?;
                let idx_val = self.eval_expr(index)?;
                match self.get_var(name)? {
                    Value::Array(items) => {
                        let len = items.borrow().len();
                        let i = to_index(&idx_val, len)?;
                        let new_value = match assign_to_binary(op) {
                            None => rhs,
                            Some(bin) => {
                                let current = items.borrow()[i].clone();
                                binary_op(&bin, current, rhs)?
                            }
                        };
                        items.borrow_mut()[i] = new_value;
                        Ok(Flow::Normal)
                    }
                    Value::Str(_) => Err("strings can't be changed in place".to_string()),
                    other => Err(format!(
                        "cannot index a value of type {}",
                        other.type_name()
                    )),
                }
            }

            Stmt::If { condition, then_block, else_block } => {
                let cond = self.eval_expr(condition)?;
                if is_truthy(&cond) {
                    self.exec_block(then_block)
                } else if let Some(else_stmts) = else_block {
                    self.exec_block(else_stmts)
                } else {
                    Ok(Flow::Normal)
                }
            }

            Stmt::While { condition, body } => {
                loop {
                    let cond = self.eval_expr(condition)?;
                    if !is_truthy(&cond) {
                        break;
                    }
                    match self.exec_block(body)? {
                        Flow::Normal | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                    }
                }
                Ok(Flow::Normal)
            }

            Stmt::For { init, condition, step, body } => {
                self.frames.last_mut().unwrap().push(HashMap::new());
                let result = self.run_for(init, condition, step, body);
                self.frames.last_mut().unwrap().pop();
                result
            }

            Stmt::ForIn { ty, var, iterable, body } => {
                let collection = self.eval_expr(iterable)?;
                let items: Vec<Value> = match collection {
                    Value::Array(items) => items.borrow().clone(),
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    other => {
                        return Err(format!(
                            "cannot loop over a value of type {}",
                            other.type_name()
                        ));
                    }
                };

                for item in items {
                    let item = coerce_to_type(ty, item, var)?;
                    let mut scope = HashMap::new();
                    scope.insert(var.clone(), item);
                    self.frames.last_mut().unwrap().push(scope);
                    let result = self.exec_stmts(body);
                    self.frames.last_mut().unwrap().pop();
                    match result? {
                        Flow::Normal | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                    }
                }
                Ok(Flow::Normal)
            }

            Stmt::Break => Ok(Flow::Break),
            Stmt::Continue => Ok(Flow::Continue),

            Stmt::Return(expr) => {
                let value = match expr {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Null,
                };
                Ok(Flow::Return(value))
            }

            Stmt::Function(f) => {
                self.functions.insert(f.name.clone(), f.clone());
                Ok(Flow::Normal)
            }

            Stmt::ExprStmt(expr) => {
                self.eval_expr(expr)?;
                Ok(Flow::Normal)
            }
        }
    }

    // ---------- expressions ----------

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Number(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::String(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Variable(name) => self.get_var(name),
            Expr::Array(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(self.eval_expr(item)?);
                }
                Ok(new_array(values))
            }
            Expr::Index { target, index } => {
                let t = self.eval_expr(target)?;
                let i = self.eval_expr(index)?;
                match t {
                    Value::Array(items) => {
                        let vec = items.borrow();
                        let idx = to_index(&i, vec.len())?;
                        Ok(vec[idx].clone())
                    }
                    Value::Str(s) => {
                        let len = s.chars().count();
                        let idx = to_index(&i, len)?;
                        Ok(Value::Str(s.chars().nth(idx).unwrap().to_string()))
                    }
                    other => Err(format!(
                        "cannot index a value of type {}",
                        other.type_name()
                    )),
                }
            }
            Expr::Unary { op, operand } => {
                let value = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => match value {
                        Value::Int(n) => n
                            .checked_neg()
                            .map(Value::Int)
                            .ok_or_else(|| "integer overflow".to_string()),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        other => Err(format!(
                            "cannot negate a value of type {}",
                            other.type_name()
                        )),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!is_truthy(&value))),
                }
            }
            Expr::Binary { left, op, right } => match op {
                // `and` / `or` short-circuit: the right side only runs if needed
                BinaryOp::And => {
                    let l = self.eval_expr(left)?;
                    if !is_truthy(&l) {
                        return Ok(Value::Bool(false));
                    }
                    let r = self.eval_expr(right)?;
                    Ok(Value::Bool(is_truthy(&r)))
                }
                BinaryOp::Or => {
                    let l = self.eval_expr(left)?;
                    if is_truthy(&l) {
                        return Ok(Value::Bool(true));
                    }
                    let r = self.eval_expr(right)?;
                    Ok(Value::Bool(is_truthy(&r)))
                }
                _ => {
                    let l = self.eval_expr(left)?;
                    let r = self.eval_expr(right)?;
                    binary_op(op, l, r)
                }
            },
            Expr::Call { module, name, args } => self.call(module.as_deref(), name, args),
        }
    }

    fn call(&mut self, module: Option<&str>, name: &str, args: &[Expr]) -> Result<Value, String> {
        let mut values = Vec::new();
        for arg in args {
            values.push(self.eval_expr(arg)?);
        }

        // Library calls like std.len(x)
        if let Some(m) = module {
            if !self.imports.contains(m) {
                return Err(format!(
                    "module '{}' is not imported (add `import {};` at the top of your file)",
                    m, m
                ));
            }
            return match m {
                "std" => std_call(name, values, &self.script_args),
                _ => Err(format!("unknown module '{}'", m)),
            };
        }

        // Built-in that is always available: print(a, b, ...)
        if name == "print" {
            let parts: Vec<String> = values.iter().map(|v| v.to_string()).collect();
            println!("{}", parts.join(" "));
            return Ok(Value::Null);
        }

        let func = match self.functions.get(name) {
            Some(f) => f.clone(),
            None => return Err(format!("undefined function '{}'", name)),
        };

        if func.params.len() != values.len() {
            return Err(format!(
                "function '{}' takes {} argument(s) but got {}",
                name,
                func.params.len(),
                values.len()
            ));
        }

        if self.frames.len() >= 200 {
            return Err("too many nested function calls (stack overflow)".to_string());
        }

        let mut scope = HashMap::new();
        for (param, value) in func.params.iter().zip(values) {
            let value = coerce_to_type(&param.ty, value, &param.name)?;
            scope.insert(param.name.clone(), value);
        }

        self.frames.push(vec![scope]);
        let result = self.exec_stmts(&func.body);
        self.frames.pop();

        match result? {
            Flow::Return(value) => Ok(value),
            Flow::Normal => Ok(Value::Null),
            Flow::Break => Err("'break' used outside of a loop".to_string()),
            Flow::Continue => Err("'continue' used outside of a loop".to_string()),
        }
    }
}

// ---------- the std library ----------

fn std_call(name: &str, args: Vec<Value>, script_args: &[String]) -> Result<Value, String> {
    let want = |n: usize| -> Result<(), String> {
        if args.len() != n {
            Err(format!(
                "std.{} takes {} argument(s) but got {}",
                name,
                n,
                args.len()
            ))
        } else {
            Ok(())
        }
    };

    match name {
        // ===== arrays and strings =====

        // std.len(array_or_str) -> int
        "len" => {
            want(1)?;
            match &args[0] {
                Value::Array(items) => Ok(Value::Int(items.borrow().len() as i64)),
                Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                other => Err(format!(
                    "std.len expects an array or str, got {}",
                    other.type_name()
                )),
            }
        }

        // std.push(array, value): adds to the end
        "push" => {
            want(2)?;
            match &args[0] {
                Value::Array(items) => {
                    items.borrow_mut().push(args[1].clone());
                    Ok(Value::Null)
                }
                other => Err(format!(
                    "std.push expects an array, got {}",
                    other.type_name()
                )),
            }
        }

        // std.pop(array) -> value: removes and returns the last element
        "pop" => {
            want(1)?;
            match &args[0] {
                Value::Array(items) => items
                    .borrow_mut()
                    .pop()
                    .ok_or_else(|| "std.pop called on an empty array".to_string()),
                other => Err(format!(
                    "std.pop expects an array, got {}",
                    other.type_name()
                )),
            }
        }

        // std.upper(str) / std.lower(str)
        "upper" => {
            want(1)?;
            match &args[0] {
                Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
                other => Err(format!(
                    "std.upper expects a str, got {}",
                    other.type_name()
                )),
            }
        }
        "lower" => {
            want(1)?;
            match &args[0] {
                Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
                other => Err(format!(
                    "std.lower expects a str, got {}",
                    other.type_name()
                )),
            }
        }

        // std.contains(str, str) or std.contains(array, value) -> bn
        "contains" => {
            want(2)?;
            match (&args[0], &args[1]) {
                (Value::Str(haystack), Value::Str(needle)) => {
                    Ok(Value::Bool(haystack.contains(needle.as_str())))
                }
                (Value::Array(items), needle) => {
                    Ok(Value::Bool(items.borrow().contains(needle)))
                }
                (first, _) => Err(format!(
                    "std.contains expects a str or an array first, got {}",
                    first.type_name()
                )),
            }
        }

        // std.split(str, separator) -> array of str
        "split" => {
            want(2)?;
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Str(sep)) => {
                    if sep.is_empty() {
                        return Err("std.split separator cannot be empty".to_string());
                    }
                    let parts: Vec<Value> = s
                        .split(sep.as_str())
                        .map(|p| Value::Str(p.to_string()))
                        .collect();
                    Ok(new_array(parts))
                }
                _ => Err("std.split expects (str, str)".to_string()),
            }
        }

        // std.join(array, separator) -> str
        "join" => {
            want(2)?;
            match (&args[0], &args[1]) {
                (Value::Array(items), Value::Str(sep)) => {
                    let parts: Vec<String> =
                        items.borrow().iter().map(|v| v.to_string()).collect();
                    Ok(Value::Str(parts.join(sep.as_str())))
                }
                _ => Err("std.join expects (array, str)".to_string()),
            }
        }

        // std.range(end) / std.range(start, end) / std.range(start, end, step) -> array of ints
        "range" => {
            let ints: Vec<i64> = args
                .iter()
                .map(|a| match a {
                    Value::Int(n) => Ok(*n),
                    other => Err(format!(
                        "std.range expects ints, got {}",
                        other.type_name()
                    )),
                })
                .collect::<Result<Vec<i64>, String>>()?;

            let (start, end, step) = match ints.as_slice() {
                [end] => (0, *end, 1),
                [start, end] => (*start, *end, 1),
                [start, end, step] => (*start, *end, *step),
                _ => return Err("std.range takes 1 to 3 arguments".to_string()),
            };
            if step == 0 {
                return Err("std.range step cannot be 0".to_string());
            }

            let mut items = Vec::new();
            let mut current = start;
            while (step > 0 && current < end) || (step < 0 && current > end) {
                if items.len() >= 10_000_000 {
                    return Err("std.range is too large".to_string());
                }
                items.push(Value::Int(current));
                current = match current.checked_add(step) {
                    Some(n) => n,
                    None => break,
                };
            }
            Ok(new_array(items))
        }

        // ===== math =====

        // std.abs(number)
        "abs" => {
            want(1)?;
            match &args[0] {
                Value::Int(n) => n
                    .checked_abs()
                    .map(Value::Int)
                    .ok_or_else(|| "integer overflow".to_string()),
                Value::Float(f) => Ok(Value::Float(f.abs())),
                other => Err(format!(
                    "std.abs expects a number, got {}",
                    other.type_name()
                )),
            }
        }

        // std.sqrt(number) -> ft
        "sqrt" => {
            want(1)?;
            match as_f64(&args[0]) {
                Some(x) if x < 0.0 => Err("std.sqrt of a negative number".to_string()),
                Some(x) => Ok(Value::Float(x.sqrt())),
                None => Err(format!(
                    "std.sqrt expects a number, got {}",
                    args[0].type_name()
                )),
            }
        }

        // std.pow(base, exponent): stays an int for int ^ non-negative int
        "pow" => {
            want(2)?;
            match (&args[0], &args[1]) {
                (Value::Int(base), Value::Int(exp)) if *exp >= 0 => {
                    let exp = u32::try_from(*exp).map_err(|_| "integer overflow".to_string())?;
                    base.checked_pow(exp)
                        .map(Value::Int)
                        .ok_or_else(|| "integer overflow".to_string())
                }
                (a, b) => match (as_f64(a), as_f64(b)) {
                    (Some(x), Some(y)) => Ok(Value::Float(x.powf(y))),
                    _ => Err("std.pow expects two numbers".to_string()),
                },
            }
        }

        // std.floor / std.ceil / std.round (number) -> int
        "floor" | "ceil" | "round" => {
            want(1)?;
            match &args[0] {
                Value::Int(n) => Ok(Value::Int(*n)),
                Value::Float(f) => {
                    let rounded = match name {
                        "floor" => f.floor(),
                        "ceil" => f.ceil(),
                        _ => f.round(),
                    };
                    float_to_int(name, rounded)
                }
                other => Err(format!(
                    "std.{} expects a number, got {}",
                    name,
                    other.type_name()
                )),
            }
        }

        // std.min(a, b) / std.max(a, b)
        "min" | "max" => {
            want(2)?;
            let pick_min = name == "min";
            match (&args[0], &args[1]) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(if pick_min {
                    (*a).min(*b)
                } else {
                    (*a).max(*b)
                })),
                (a, b) => match (as_f64(a), as_f64(b)) {
                    (Some(x), Some(y)) => Ok(Value::Float(if pick_min {
                        x.min(y)
                    } else {
                        x.max(y)
                    })),
                    _ => Err(format!("std.{} expects two numbers", name)),
                },
            }
        }

        // ===== conversions =====

        "to_int" => {
            want(1)?;
            match &args[0] {
                Value::Int(n) => Ok(Value::Int(*n)),
                Value::Float(f) => float_to_int("to_int", f.trunc()),
                Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                Value::Str(s) => s
                    .trim()
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| format!("std.to_int cannot convert \"{}\" to an int", s)),
                other => Err(format!(
                    "std.to_int cannot convert a value of type {}",
                    other.type_name()
                )),
            }
        }

        "to_float" => {
            want(1)?;
            match &args[0] {
                Value::Int(n) => Ok(Value::Float(*n as f64)),
                Value::Float(f) => Ok(Value::Float(*f)),
                Value::Str(s) => s
                    .trim()
                    .parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| format!("std.to_float cannot convert \"{}\" to a float", s)),
                other => Err(format!(
                    "std.to_float cannot convert a value of type {}",
                    other.type_name()
                )),
            }
        }

        // std.to_str(anything) -> str
        "to_str" => {
            want(1)?;
            Ok(Value::Str(args[0].to_string()))
        }

        // ===== input, files and the system =====

        // std.input() or std.input(prompt) -> str
        "input" => {
            if args.len() > 1 {
                return Err("std.input takes 0 or 1 argument(s)".to_string());
            }
            if let Some(prompt) = args.first() {
                print!("{}", prompt);
                let _ = io::stdout().flush();
            }
            let mut line = String::new();
            io::stdin()
                .read_line(&mut line)
                .map_err(|e| format!("std.input failed: {}", e))?;
            while line.ends_with('\n') || line.ends_with('\r') {
                line.pop();
            }
            Ok(Value::Str(line))
        }

        // std.read_file(path) -> str
        "read_file" => {
            want(1)?;
            match &args[0] {
                Value::Str(path) => fs::read_to_string(path)
                    .map(Value::Str)
                    .map_err(|e| format!("std.read_file could not read '{}': {}", path, e)),
                other => Err(format!(
                    "std.read_file expects a str path, got {}",
                    other.type_name()
                )),
            }
        }

        // std.write_file(path, text): replaces the file
        "write_file" => {
            want(2)?;
            match (&args[0], &args[1]) {
                (Value::Str(path), Value::Str(text)) => fs::write(path, text)
                    .map(|_| Value::Null)
                    .map_err(|e| format!("std.write_file could not write '{}': {}", path, e)),
                _ => Err("std.write_file expects (str, str)".to_string()),
            }
        }

        // std.append_file(path, text): adds to the end, creating the file if needed
        "append_file" => {
            want(2)?;
            match (&args[0], &args[1]) {
                (Value::Str(path), Value::Str(text)) => {
                    let mut file = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .map_err(|e| {
                            format!("std.append_file could not open '{}': {}", path, e)
                        })?;
                    file.write_all(text.as_bytes()).map_err(|e| {
                        format!("std.append_file could not write '{}': {}", path, e)
                    })?;
                    Ok(Value::Null)
                }
                _ => Err("std.append_file expects (str, str)".to_string()),
            }
        }

        // std.file_exists(path) -> bn
        "file_exists" => {
            want(1)?;
            match &args[0] {
                Value::Str(path) => Ok(Value::Bool(fs::metadata(path).is_ok())),
                other => Err(format!(
                    "std.file_exists expects a str path, got {}",
                    other.type_name()
                )),
            }
        }

        // std.lines(str) -> array of str
        "lines" => {
            want(1)?;
            match &args[0] {
                Value::Str(s) => Ok(new_array(
                    s.lines().map(|l| Value::Str(l.to_string())).collect(),
                )),
                other => Err(format!(
                    "std.lines expects a str, got {}",
                    other.type_name()
                )),
            }
        }

        // std.args() -> array of str (everything after the file in `wave run file.td ...`)
        "args" => {
            want(0)?;
            Ok(new_array(
                script_args.iter().map(|a| Value::Str(a.clone())).collect(),
            ))
        }

        // std.exit(code): stops the program
        "exit" => {
            want(1)?;
            match &args[0] {
                Value::Int(code) => {
                    let _ = io::stdout().flush();
                    process::exit(*code as i32)
                }
                other => Err(format!(
                    "std.exit expects an int, got {}",
                    other.type_name()
                )),
            }
        }

        _ => Err(format!("std has no function named '{}'", name)),
    }
}

// ---------- helpers ----------

fn as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Int(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn float_to_int(name: &str, f: f64) -> Result<Value, String> {
    if f.is_finite() && f >= -9.2e18 && f <= 9.2e18 {
        Ok(Value::Int(f as i64))
    } else {
        Err(format!("std.{} result is out of integer range", name))
    }
}

fn assign_to_binary(op: &AssignOp) -> Option<BinaryOp> {
    match op {
        AssignOp::Assign => None,
        AssignOp::AddAssign => Some(BinaryOp::Add),
        AssignOp::SubAssign => Some(BinaryOp::Sub),
        AssignOp::MulAssign => Some(BinaryOp::Mul),
        AssignOp::DivAssign => Some(BinaryOp::Div),
        AssignOp::ModAssign => Some(BinaryOp::Mod),
    }
}

// Checks that an index is a whole number inside 0..len
fn to_index(index: &Value, len: usize) -> Result<usize, String> {
    match index {
        Value::Int(n) => {
            if *n < 0 || *n as usize >= len {
                Err(format!("index {} is out of range for length {}", n, len))
            } else {
                Ok(*n as usize)
            }
        }
        other => Err(format!("index must be an int, got {}", other.type_name())),
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Int(n) => *n != 0,
        Value::Float(f) => *f != 0.0,
        Value::Bool(b) => *b,
        Value::Str(s) => !s.is_empty(),
        Value::Array(items) => !items.borrow().is_empty(),
        Value::Null => false,
    }
}

// Checks a value against a declared type. An int is allowed where an ft is
// declared and gets widened: `ft x = 5;` stores 5.0
fn coerce_to_type(ty: &Type, value: Value, name: &str) -> Result<Value, String> {
    if let (Type::Ft, Value::Int(n)) = (ty, &value) {
        return Ok(Value::Float(*n as f64));
    }

    let (ok, expected) = match ty {
        Type::Int => (matches!(value, Value::Int(_)), "int"),
        Type::Ft => (matches!(value, Value::Float(_)), "ft"),
        Type::Bn => (matches!(value, Value::Bool(_)), "bn"),
        Type::Str => (matches!(value, Value::Str(_)), "str"),
        Type::Array => (matches!(value, Value::Array(_)), "array"),
    };

    if ok {
        Ok(value)
    } else {
        Err(format!(
            "'{}' is declared as {} but got a value of type {}",
            name,
            expected,
            value.type_name()
        ))
    }
}

fn float_op(op: &BinaryOp, a: f64, b: f64) -> Result<Value, String> {
    match op {
        BinaryOp::Add => Ok(Value::Float(a + b)),
        BinaryOp::Sub => Ok(Value::Float(a - b)),
        BinaryOp::Mul => Ok(Value::Float(a * b)),
        BinaryOp::Div => {
            if b == 0.0 {
                Err("division by zero".to_string())
            } else {
                Ok(Value::Float(a / b))
            }
        }
        BinaryOp::Mod => {
            if b == 0.0 {
                Err("modulo by zero".to_string())
            } else {
                Ok(Value::Float(a % b))
            }
        }
        BinaryOp::Eq => Ok(Value::Bool(a == b)),
        BinaryOp::NotEq => Ok(Value::Bool(a != b)),
        BinaryOp::Less => Ok(Value::Bool(a < b)),
        BinaryOp::Greater => Ok(Value::Bool(a > b)),
        BinaryOp::LessEq => Ok(Value::Bool(a <= b)),
        BinaryOp::GreaterEq => Ok(Value::Bool(a >= b)),
        BinaryOp::And | BinaryOp::Or => Err(format!("cannot apply {:?} to numbers", op)),
    }
}

fn binary_op(op: &BinaryOp, left: Value, right: Value) -> Result<Value, String> {
    // Anything with a float on one side and a number on the other is done in floats
    match (&left, &right) {
        (Value::Float(_), Value::Float(_))
        | (Value::Float(_), Value::Int(_))
        | (Value::Int(_), Value::Float(_)) => {
            let a = as_f64(&left).unwrap();
            let b = as_f64(&right).unwrap();
            return float_op(op, a, b);
        }
        _ => {}
    }

    match (op, left, right) {
        // ----- math on ints -----
        (BinaryOp::Add, Value::Int(a), Value::Int(b)) => a
            .checked_add(b)
            .map(Value::Int)
            .ok_or_else(|| "integer overflow".to_string()),
        (BinaryOp::Sub, Value::Int(a), Value::Int(b)) => a
            .checked_sub(b)
            .map(Value::Int)
            .ok_or_else(|| "integer overflow".to_string()),
        (BinaryOp::Mul, Value::Int(a), Value::Int(b)) => a
            .checked_mul(b)
            .map(Value::Int)
            .ok_or_else(|| "integer overflow".to_string()),
        (BinaryOp::Div, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err("division by zero".to_string());
            }
            a.checked_div(b)
                .map(Value::Int)
                .ok_or_else(|| "integer overflow".to_string())
        }
        (BinaryOp::Mod, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err("modulo by zero".to_string());
            }
            a.checked_rem(b)
                .map(Value::Int)
                .ok_or_else(|| "integer overflow".to_string())
        }

        // ----- joining -----
        (BinaryOp::Add, Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
        (BinaryOp::Add, Value::Array(a), Value::Array(b)) => {
            let mut joined = a.borrow().clone();
            joined.extend(b.borrow().iter().cloned());
            Ok(new_array(joined))
        }

        // ----- equality works on any two values -----
        (BinaryOp::Eq, a, b) => Ok(Value::Bool(a == b)),
        (BinaryOp::NotEq, a, b) => Ok(Value::Bool(a != b)),

        // ----- ordering: ints with ints, strings with strings -----
        (BinaryOp::Less, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
        (BinaryOp::Greater, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
        (BinaryOp::LessEq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
        (BinaryOp::GreaterEq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
        (BinaryOp::Less, Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a < b)),
        (BinaryOp::Greater, Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a > b)),
        (BinaryOp::LessEq, Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a <= b)),
        (BinaryOp::GreaterEq, Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a >= b)),

        (op, a, b) => Err(format!(
            "cannot apply {:?} to {} and {}",
            op,
            a.type_name(),
            b.type_name()
        )),
    }
}