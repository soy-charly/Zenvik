use crate::ast::{Expr, ExprKind, Stmt, StmtKind};
use std::collections::{HashMap, HashSet};

const BUILTINS: &[&str] = &[
    "print", "input", "len", "range", "str", "int", "float", "bool", "type",
];

pub fn check(statements: &[Stmt]) -> Result<(), String> {
    let mut functions = HashSet::new();
    let mut signatures = HashMap::new();
    for statement in statements {
        if let StmtKind::Function {
            name,
            parameters,
            return_type,
            ..
        } = &statement.kind
        {
            if BUILTINS.contains(&name.as_str()) || !functions.insert(name) {
                return Err(diagnostic(
                    &statement.position,
                    format!("function `{name}` is already defined or reserved"),
                ));
            }
            let mut seen = HashSet::new();
            if parameters
                .iter()
                .any(|parameter| !seen.insert(&parameter.name))
            {
                return Err(diagnostic(
                    &statement.position,
                    format!("function `{name}` has a duplicate parameter"),
                ));
            }
            for type_name in parameters
                .iter()
                .filter_map(|parameter| parameter.type_name.as_deref())
                .chain(return_type.as_deref())
            {
                if !valid_type(type_name) {
                    return Err(diagnostic(
                        &statement.position,
                        format!("unknown type `{type_name}`"),
                    ));
                }
            }
            signatures.insert(name.clone(), FunctionSignature::from_statement(statement));
        }
    }

    let mut checker = Checker {
        functions: &functions,
        function_depth: 0,
        control_depth: 0,
        loop_depth: 0,
    };
    checker.statements(statements)?;
    let mut type_checker = TypeChecker {
        functions: &signatures,
        return_type: None,
    };
    type_checker.statements(statements, &mut HashMap::new())
}

struct Checker<'a> {
    functions: &'a HashSet<&'a String>,
    function_depth: usize,
    control_depth: usize,
    loop_depth: usize,
}

impl Checker<'_> {
    fn control_statements(&mut self, statements: &[Stmt]) -> Result<(), String> {
        self.control_depth += 1;
        let result = self.statements(statements);
        self.control_depth -= 1;
        result
    }

    fn statements(&mut self, statements: &[Stmt]) -> Result<(), String> {
        for statement in statements {
            match &statement.kind {
                StmtKind::Import(_) => {
                    return Err(diagnostic(
                        &statement.position,
                        "imports are only supported at top level".into(),
                    ));
                }
                StmtKind::Expr(expression) => self.expression(expression)?,
                StmtKind::Assign { value, .. } => self.expression(value)?,
                StmtKind::If {
                    branches,
                    otherwise,
                } => {
                    for (condition, body) in branches {
                        self.expression(condition)?;
                        self.control_statements(body)?;
                    }
                    self.control_statements(otherwise)?;
                }
                StmtKind::While { condition, body } => {
                    self.expression(condition)?;
                    self.loop_depth += 1;
                    self.control_statements(body)?;
                    self.loop_depth -= 1;
                }
                StmtKind::For { iterable, body, .. } => {
                    self.expression(iterable)?;
                    self.loop_depth += 1;
                    self.control_statements(body)?;
                    self.loop_depth -= 1;
                }
                StmtKind::Function { body, .. } => {
                    if self.function_depth > 0 {
                        return Err(diagnostic(
                            &statement.position,
                            "nested function definitions are not supported".into(),
                        ));
                    }
                    if self.control_depth > 0 {
                        return Err(diagnostic(
                            &statement.position,
                            "function definitions must be at top level".into(),
                        ));
                    }
                    self.function_depth += 1;
                    self.statements(body)?;
                    self.function_depth -= 1;
                }
                StmtKind::Return(value) => {
                    if self.function_depth == 0 {
                        return Err(diagnostic(
                            &statement.position,
                            "`return` is only valid inside a function".into(),
                        ));
                    }
                    if let Some(value) = value {
                        self.expression(value)?;
                    }
                }
                StmtKind::Break | StmtKind::Continue if self.loop_depth == 0 => {
                    let keyword = if matches!(statement.kind, StmtKind::Break) {
                        "break"
                    } else {
                        "continue"
                    };
                    return Err(diagnostic(
                        &statement.position,
                        format!("`{keyword}` is only valid inside a loop"),
                    ));
                }
                StmtKind::Break | StmtKind::Continue => {}
            }
        }
        Ok(())
    }

    fn expression(&self, expression: &Expr) -> Result<(), String> {
        match &expression.kind {
            ExprKind::Unary { operand, .. } => self.expression(operand)?,
            ExprKind::Binary { left, right, .. } => {
                self.expression(left)?;
                self.expression(right)?;
            }
            ExprKind::Call { callee, arguments } => {
                let ExprKind::Name(name) = &callee.kind else {
                    return Err(diagnostic(
                        &callee.position,
                        "only named functions can be called".into(),
                    ));
                };
                if !BUILTINS.contains(&name.as_str())
                    && !self
                        .functions
                        .iter()
                        .any(|function| function.as_str() == name)
                {
                    return Err(diagnostic(
                        &callee.position,
                        format!("unknown function `{name}`"),
                    ));
                }
                for argument in arguments {
                    self.expression(argument)?;
                }
            }
            ExprKind::List(items) => {
                for item in items {
                    self.expression(item)?;
                }
            }
            ExprKind::Dict(items) => {
                for (key, value) in items {
                    self.expression(key)?;
                    self.expression(value)?;
                }
            }
            ExprKind::Index { value, index } => {
                self.expression(value)?;
                self.expression(index)?;
            }
            ExprKind::Slice {
                value,
                start,
                end,
                step,
            } => {
                self.expression(value)?;
                for expression in [start, end, step].into_iter().flatten() {
                    self.expression(expression)?;
                }
            }
            ExprKind::None
            | ExprKind::Bool(_)
            | ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Name(_) => {}
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StaticType {
    Any,
    None,
    Bool,
    Char,
    Integer,
    Float,
    String,
    List,
    Dict,
    Void,
}

struct FunctionSignature {
    parameters: Vec<StaticType>,
    return_type: StaticType,
}

impl FunctionSignature {
    fn from_statement(statement: &Stmt) -> Self {
        let StmtKind::Function {
            parameters,
            return_type,
            ..
        } = &statement.kind
        else {
            unreachable!("function signature requires a function statement");
        };
        Self {
            parameters: parameters
                .iter()
                .map(|parameter| {
                    parameter
                        .type_name
                        .as_deref()
                        .map(static_type)
                        .unwrap_or(StaticType::Any)
                })
                .collect(),
            return_type: return_type
                .as_deref()
                .map(static_type)
                .unwrap_or(StaticType::Any),
        }
    }
}

struct TypeChecker<'a> {
    functions: &'a HashMap<String, FunctionSignature>,
    return_type: Option<StaticType>,
}

impl TypeChecker<'_> {
    fn statements(
        &mut self,
        statements: &[Stmt],
        environment: &mut HashMap<String, StaticType>,
    ) -> Result<(), String> {
        for statement in statements {
            match &statement.kind {
                StmtKind::Expr(expression) => {
                    self.expression_type(expression, environment)?;
                }
                StmtKind::Assign {
                    name,
                    type_name,
                    operator,
                    value,
                } => {
                    let value_type = self.expression_type(value, environment)?;
                    if let Some(type_name) = type_name {
                        if !valid_type(type_name) {
                            return Err(diagnostic(
                                &statement.position,
                                format!("unknown type `{type_name}`"),
                            ));
                        }
                        let target_type = static_type(type_name);
                        if target_type == StaticType::Void {
                            return Err(diagnostic(
                                &statement.position,
                                "`void` cannot be used for a variable".into(),
                            ));
                        }
                        if environment.contains_key(name) {
                            return Err(diagnostic(
                                &statement.position,
                                format!("variable `{name}` is already declared"),
                            ));
                        }
                        self.check_assignment(&statement.position, name, target_type, value_type)?;
                        environment.insert(name.clone(), target_type);
                    } else if operator == "=" {
                        if let Some(existing) = environment.get(name).copied() {
                            self.check_assignment(&statement.position, name, existing, value_type)?;
                            if existing != StaticType::Any
                                && value_type != StaticType::Any
                                && is_numeric(existing)
                                && is_numeric(value_type)
                            {
                                environment.insert(name.clone(), common_type(existing, value_type));
                            }
                        } else {
                            environment.insert(name.clone(), value_type);
                        }
                    } else {
                        let existing = environment.get(name).copied().unwrap_or(StaticType::Any);
                        let result_type = self.binary_type(
                            &statement.position,
                            operator.trim_end_matches('='),
                            existing,
                            value_type,
                        )?;
                        self.check_assignment(&statement.position, name, existing, result_type)?;
                    }
                }
                StmtKind::If {
                    branches,
                    otherwise,
                } => {
                    let original = environment.clone();
                    let mut paths = Vec::new();
                    for (condition, body) in branches {
                        self.expression_type(condition, environment)?;
                        let mut branch = original.clone();
                        self.statements(body, &mut branch)?;
                        paths.push(branch);
                    }
                    if otherwise.is_empty() {
                        paths.push(original);
                    } else {
                        let mut branch = original;
                        self.statements(otherwise, &mut branch)?;
                        paths.push(branch);
                    }
                    *environment = merge_environments(&paths);
                }
                StmtKind::While { condition, body } => {
                    self.expression_type(condition, environment)?;
                    let mut loop_environment = environment.clone();
                    self.statements(body, &mut loop_environment)?;
                    *environment = merge_environments(&[environment.clone(), loop_environment]);
                }
                StmtKind::For {
                    name,
                    iterable,
                    body,
                } => {
                    self.expression_type(iterable, environment)?;
                    let mut loop_environment = environment.clone();
                    loop_environment.insert(name.clone(), StaticType::Any);
                    self.statements(body, &mut loop_environment)?;
                    *environment = merge_environments(&[environment.clone(), loop_environment]);
                }
                StmtKind::Function {
                    name,
                    parameters,
                    return_type: _,
                    body,
                } => {
                    let signature = self
                        .functions
                        .get(name)
                        .expect("all function signatures are collected before checking");
                    let mut function_environment = HashMap::new();
                    for (parameter, parameter_type) in parameters.iter().zip(&signature.parameters)
                    {
                        function_environment.insert(parameter.name.clone(), *parameter_type);
                    }
                    let previous_return_type = self.return_type;
                    self.return_type = Some(signature.return_type);
                    self.statements(body, &mut function_environment)?;
                    if signature.return_type != StaticType::Any
                        && signature.return_type != StaticType::Void
                        && !always_returns(body)
                    {
                        return Err(diagnostic(
                            &statement.position,
                            format!("function `{name}` does not return a value on every path"),
                        ));
                    }
                    self.return_type = previous_return_type;
                }
                StmtKind::Return(value) => {
                    if let Some(expected) = self.return_type {
                        match (expected, value) {
                            (StaticType::Void, Some(value)) => {
                                self.expression_type(value, environment)?;
                                return Err(diagnostic(
                                    &statement.position,
                                    "`void` function cannot return a value".into(),
                                ));
                            }
                            (expected, None) if expected != StaticType::Void => {
                                return Err(diagnostic(
                                    &statement.position,
                                    "non-void function must return a value".into(),
                                ));
                            }
                            (expected, Some(value)) => {
                                let actual = self.expression_type(value, environment)?;
                                self.check_assignment(
                                    &statement.position,
                                    "return value",
                                    expected,
                                    actual,
                                )?;
                            }
                            (StaticType::Void, None) | (StaticType::Any, _) => {}
                            (_, None) => unreachable!("non-void returns are rejected above"),
                        }
                    } else if let Some(value) = value {
                        self.expression_type(value, environment)?;
                    }
                }
                StmtKind::Break | StmtKind::Continue => {}
            }
        }
        Ok(())
    }

    fn expression_type(
        &self,
        expression: &Expr,
        environment: &HashMap<String, StaticType>,
    ) -> Result<StaticType, String> {
        match &expression.kind {
            ExprKind::None => Ok(StaticType::None),
            ExprKind::Bool(_) => Ok(StaticType::Bool),
            ExprKind::Number(number) => Ok(if number.contains('.') {
                StaticType::Float
            } else {
                StaticType::Integer
            }),
            ExprKind::String(_) => Ok(StaticType::String),
            ExprKind::Name(name) => Ok(environment.get(name).copied().unwrap_or(StaticType::Any)),
            ExprKind::List(items) => {
                for item in items {
                    self.expression_type(item, environment)?;
                }
                Ok(StaticType::List)
            }
            ExprKind::Dict(items) => {
                for (key, value) in items {
                    self.expression_type(key, environment)?;
                    self.expression_type(value, environment)?;
                }
                Ok(StaticType::Dict)
            }
            ExprKind::Unary { operator, operand } => {
                let operand_type = self.expression_type(operand, environment)?;
                match operator.as_str() {
                    "not" => Ok(StaticType::Bool),
                    "+" | "-" if operand_type == StaticType::Any || is_numeric(operand_type) => {
                        Ok(operand_type)
                    }
                    "+" | "-" => Err(diagnostic(
                        &expression.position,
                        format!("unary `{operator}` requires a numeric value"),
                    )),
                    _ => unreachable!("parser only creates supported unary operators"),
                }
            }
            ExprKind::Binary {
                left,
                operator,
                right,
            } => {
                let left_type = self.expression_type(left, environment)?;
                let right_type = self.expression_type(right, environment)?;
                self.binary_type(&expression.position, operator, left_type, right_type)
            }
            ExprKind::Call { callee, arguments } => {
                let ExprKind::Name(name) = &callee.kind else {
                    return Ok(StaticType::Any);
                };
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.expression_type(argument, environment))
                    .collect::<Result<Vec<_>, _>>()?;
                if let Some(signature) = self.functions.get(name) {
                    if argument_types.len() != signature.parameters.len() {
                        return Err(diagnostic(
                            &expression.position,
                            format!(
                                "function `{name}` expects {} argument(s), got {}",
                                signature.parameters.len(),
                                argument_types.len()
                            ),
                        ));
                    }
                    for (index, (actual, expected)) in
                        argument_types.iter().zip(&signature.parameters).enumerate()
                    {
                        self.check_assignment(
                            &arguments[index].position,
                            &format!("argument {}", index + 1),
                            *expected,
                            *actual,
                        )?;
                    }
                    return Ok(signature.return_type);
                }
                Ok(match name.as_str() {
                    "print" => StaticType::None,
                    "input" | "str" | "type" => StaticType::String,
                    "len" | "int" => StaticType::Integer,
                    "float" => StaticType::Float,
                    "bool" => StaticType::Bool,
                    "range" => StaticType::List,
                    _ => StaticType::Any,
                })
            }
            ExprKind::Index { value, index } => {
                self.expression_type(value, environment)?;
                self.expression_type(index, environment)?;
                Ok(StaticType::Any)
            }
            ExprKind::Slice {
                value,
                start,
                end,
                step,
            } => {
                self.expression_type(value, environment)?;
                for bound in [start, end, step].into_iter().flatten() {
                    self.expression_type(bound, environment)?;
                }
                Ok(StaticType::Any)
            }
        }
    }

    fn binary_type(
        &self,
        position: &crate::ast::Position,
        operator: &str,
        left: StaticType,
        right: StaticType,
    ) -> Result<StaticType, String> {
        match operator {
            "==" | "!=" | "<" | "<=" | ">" | ">=" | "in" => Ok(StaticType::Bool),
            "and" | "or" => Ok(common_type(left, right)),
            "+" if left == StaticType::String || right == StaticType::String => {
                Ok(StaticType::String)
            }
            "+" if left == StaticType::List && right == StaticType::List => Ok(StaticType::List),
            "+" | "-" | "*" | "/" | "%" | "**" => {
                if left == StaticType::Any || right == StaticType::Any {
                    return Ok(StaticType::Any);
                }
                if !is_numeric(left) || !is_numeric(right) {
                    return Err(diagnostic(
                        position,
                        format!("operator `{operator}` requires numeric operands"),
                    ));
                }
                if operator == "/" {
                    Ok(StaticType::Float)
                } else {
                    Ok(common_type(left, right))
                }
            }
            _ => Ok(StaticType::Any),
        }
    }

    fn check_assignment(
        &self,
        position: &crate::ast::Position,
        name: &str,
        target: StaticType,
        source: StaticType,
    ) -> Result<(), String> {
        if target == StaticType::Any
            || source == StaticType::Any
            || target == source
            || (is_numeric(target) && is_numeric(source))
            || (target == StaticType::Char && source == StaticType::String)
        {
            Ok(())
        } else {
            Err(diagnostic(
                position,
                format!(
                    "cannot assign `{}` to `{}` variable `{name}`",
                    type_label(source),
                    type_label(target)
                ),
            ))
        }
    }
}

fn merge_environments(paths: &[HashMap<String, StaticType>]) -> HashMap<String, StaticType> {
    let mut merged = HashMap::new();
    for path in paths {
        for (name, value_type) in path {
            merged
                .entry(name.clone())
                .and_modify(|existing| *existing = common_type(*existing, *value_type))
                .or_insert(*value_type);
        }
    }
    merged
}

fn always_returns(statements: &[Stmt]) -> bool {
    statements.iter().any(|statement| match &statement.kind {
        StmtKind::Return(_) => true,
        StmtKind::If {
            branches,
            otherwise,
        } => {
            !otherwise.is_empty()
                && branches.iter().all(|(_, body)| always_returns(body))
                && always_returns(otherwise)
        }
        _ => false,
    })
}

fn valid_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "void"
            | "bool"
            | "char"
            | "signed char"
            | "unsigned char"
            | "short int"
            | "unsigned short int"
            | "int"
            | "unsigned int"
            | "long int"
            | "unsigned long int"
            | "long long int"
            | "unsigned long long int"
            | "float"
            | "double"
            | "long double"
            | "wchar_t"
            | "char8_t"
            | "char16_t"
            | "char32_t"
            | "string"
            | "int8_t"
            | "uint8_t"
            | "int16_t"
            | "uint16_t"
            | "int32_t"
            | "uint32_t"
            | "int64_t"
            | "uint64_t"
            | "intptr_t"
            | "uintptr_t"
            | "intmax_t"
            | "uintmax_t"
            | "size_t"
            | "ptrdiff_t"
    )
}

fn static_type(type_name: &str) -> StaticType {
    match type_name {
        "void" => StaticType::Void,
        "bool" => StaticType::Bool,
        "char" | "signed char" | "unsigned char" | "wchar_t" | "char8_t" | "char16_t"
        | "char32_t" => StaticType::Char,
        "float" | "double" | "long double" => StaticType::Float,
        "string" => StaticType::String,
        "short int"
        | "unsigned short int"
        | "int"
        | "unsigned int"
        | "long int"
        | "unsigned long int"
        | "long long int"
        | "unsigned long long int"
        | "int8_t"
        | "uint8_t"
        | "int16_t"
        | "uint16_t"
        | "int32_t"
        | "uint32_t"
        | "int64_t"
        | "uint64_t"
        | "intptr_t"
        | "uintptr_t"
        | "intmax_t"
        | "uintmax_t"
        | "size_t"
        | "ptrdiff_t" => StaticType::Integer,
        _ => StaticType::Any,
    }
}

fn common_type(left: StaticType, right: StaticType) -> StaticType {
    if left == right {
        left
    } else if left == StaticType::Any || right == StaticType::Any {
        StaticType::Any
    } else if is_numeric(left) && is_numeric(right) {
        if left == StaticType::Float || right == StaticType::Float {
            StaticType::Float
        } else {
            StaticType::Integer
        }
    } else {
        StaticType::Any
    }
}

fn is_numeric(value_type: StaticType) -> bool {
    matches!(
        value_type,
        StaticType::Bool | StaticType::Char | StaticType::Integer | StaticType::Float
    )
}

fn type_label(value_type: StaticType) -> &'static str {
    match value_type {
        StaticType::Any => "dynamic",
        StaticType::None => "None",
        StaticType::Bool => "bool",
        StaticType::Char => "char",
        StaticType::Integer => "integer",
        StaticType::Float => "floating-point",
        StaticType::String => "string",
        StaticType::List => "list",
        StaticType::Dict => "dictionary",
        StaticType::Void => "void",
    }
}

fn diagnostic(position: &crate::ast::Position, message: String) -> String {
    format!(
        "{}:{}:{}: error: {}",
        position.file, position.line, position.column, message
    )
}
