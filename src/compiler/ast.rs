use std::fmt;

/// Represents the entire program
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// Statement types in AbbyScript
#[derive(Debug, Clone)]
pub enum Statement {
    VarDecl(VarDecl),
    FuncDecl(FuncDecl),
    ExprStmt(ExprStmt),
    IfStmt(IfStmt),
    WhileStmt(WhileStmt),
    ReturnStmt(ReturnStmt),
    Block(Block),
}

/// Expression types in AbbyScript
#[derive(Debug, Clone)]
pub enum Expression {
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    Assignment(AssignmentExpr),
    Variable(VariableExpr),
    Literal(LiteralExpr),
    MemberAccess(MemberAccessExpr),
    StorageAccess(StorageAccessExpr),
    MemoryAccess(MemoryAccessExpr),
    ArrayAccess(ArrayAccessExpr),
    MemoryAssignment(MemoryAssignmentExpr),
    StorageArrayAssignment(StorageArrayAssignmentExpr),
}

/// Variable declaration: let x = expression;
#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub initializer: Expression,
}

/// Function declaration: function name(params) { body }
#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Block,
}

/// Expression statement: expression;
#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expression: Expression,
}

/// If statement: if (condition) then_branch else else_branch
#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_branch: Box<Statement>,
    pub else_branch: Option<Box<Statement>>,
}

/// While loop: while (condition) body
#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expression,
    pub body: Box<Statement>,
}

/// Return statement: return expression?;
#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expression>,
}

/// Block: { statements }
#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
}

/// Binary expression: left operator right
#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<Expression>,
    pub operator: BinaryOperator,
    pub right: Box<Expression>,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
}

/// Unary expression: operator operand
#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Minus,
    Not,
}

/// Function call: callee(arguments)
#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
}

/// Assignment: name = value
#[derive(Debug, Clone)]
pub struct AssignmentExpr {
    pub name: String,
    pub value: Box<Expression>,
}

/// Variable reference: name
#[derive(Debug, Clone)]
pub struct VariableExpr {
    pub name: String,
}

/// Literal values
#[derive(Debug, Clone)]
pub enum LiteralExpr {
    Number(u64),
    Boolean(bool),
    String(String),
    Array(Vec<Expression>),
}

/// Member access: object.property
#[derive(Debug, Clone)]
pub struct MemberAccessExpr {
    pub object: Box<Expression>,
    pub property: String,
}

/// Storage access: storage.get(key) or storage.set(key, value)
#[derive(Debug, Clone)]
pub enum StorageAccessExpr {
    Get(Box<Expression>),                  // storage.get(key)
    Set(Box<Expression>, Box<Expression>), // storage.set(key, value)
}

/// Memory access: memory.load(offset) or memory.store(offset, value)
#[derive(Debug, Clone)]
pub enum MemoryAccessExpr {
    Load(Box<Expression>),                   // memory.load(offset)
    Store(Box<Expression>, Box<Expression>), // memory.store(offset, value)
}

/// Array access: object[index]
#[derive(Debug, Clone)]
pub struct ArrayAccessExpr {
    pub object: Box<Expression>,
    pub index: Box<Expression>,
}

/// Memory assignment: memory = value (implicit allocation)
#[derive(Debug, Clone)]
pub struct MemoryAssignmentExpr {
    pub value: Box<Expression>,
}

/// Storage array assignment: storage[index] = value
#[derive(Debug, Clone)]
pub struct StorageArrayAssignmentExpr {
    pub index: Box<Expression>,
    pub value: Box<Expression>,
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOperator::Add => write!(f, "+"),
            BinaryOperator::Subtract => write!(f, "-"),
            BinaryOperator::Multiply => write!(f, "*"),
            BinaryOperator::Divide => write!(f, "/"),
            BinaryOperator::Modulo => write!(f, "%"),
            BinaryOperator::Equal => write!(f, "=="),
            BinaryOperator::NotEqual => write!(f, "!="),
            BinaryOperator::Greater => write!(f, ">"),
            BinaryOperator::GreaterEqual => write!(f, ">="),
            BinaryOperator::Less => write!(f, "<"),
            BinaryOperator::LessEqual => write!(f, "<="),
            BinaryOperator::And => write!(f, "&&"),
            BinaryOperator::Or => write!(f, "||"),
        }
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOperator::Minus => write!(f, "-"),
            UnaryOperator::Not => write!(f, "!"),
        }
    }
}

// Helper functions for AST construction
impl Expression {
    pub fn binary(left: Expression, operator: BinaryOperator, right: Expression) -> Self {
        Expression::Binary(BinaryExpr {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
    }

    pub fn unary(operator: UnaryOperator, operand: Expression) -> Self {
        Expression::Unary(UnaryExpr {
            operator,
            operand: Box::new(operand),
        })
    }

    pub fn call(callee: Expression, arguments: Vec<Expression>) -> Self {
        Expression::Call(CallExpr {
            callee: Box::new(callee),
            arguments,
        })
    }

    pub fn assignment(name: String, value: Expression) -> Self {
        Expression::Assignment(AssignmentExpr {
            name,
            value: Box::new(value),
        })
    }

    pub fn variable(name: String) -> Self {
        Expression::Variable(VariableExpr { name })
    }

    pub fn number(value: u64) -> Self {
        Expression::Literal(LiteralExpr::Number(value))
    }

    pub fn boolean(value: bool) -> Self {
        Expression::Literal(LiteralExpr::Boolean(value))
    }

    pub fn string(value: String) -> Self {
        Expression::Literal(LiteralExpr::String(value))
    }

    pub fn array(elements: Vec<Expression>) -> Self {
        Expression::Literal(LiteralExpr::Array(elements))
    }

    pub fn array_access(object: Expression, index: Expression) -> Self {
        Expression::ArrayAccess(ArrayAccessExpr {
            object: Box::new(object),
            index: Box::new(index),
        })
    }

    pub fn member_access(object: Expression, property: String) -> Self {
        Expression::MemberAccess(MemberAccessExpr {
            object: Box::new(object),
            property,
        })
    }

    pub fn memory_assignment(value: Expression) -> Self {
        Expression::MemoryAssignment(MemoryAssignmentExpr {
            value: Box::new(value),
        })
    }

    pub fn storage_array_assignment(index: Expression, value: Expression) -> Self {
        Expression::StorageArrayAssignment(StorageArrayAssignmentExpr {
            index: Box::new(index),
            value: Box::new(value),
        })
    }
}

impl Statement {
    pub fn var_decl(name: String, initializer: Expression) -> Self {
        Statement::VarDecl(VarDecl { name, initializer })
    }

    pub fn func_decl(name: String, params: Vec<String>, body: Block) -> Self {
        Statement::FuncDecl(FuncDecl { name, params, body })
    }

    pub fn expr_stmt(expression: Expression) -> Self {
        Statement::ExprStmt(ExprStmt { expression })
    }

    pub fn if_stmt(
        condition: Expression,
        then_branch: Statement,
        else_branch: Option<Statement>,
    ) -> Self {
        Statement::IfStmt(IfStmt {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        })
    }

    pub fn while_stmt(condition: Expression, body: Statement) -> Self {
        Statement::WhileStmt(WhileStmt {
            condition,
            body: Box::new(body),
        })
    }

    pub fn return_stmt(value: Option<Expression>) -> Self {
        Statement::ReturnStmt(ReturnStmt { value })
    }

    pub fn block(statements: Vec<Statement>) -> Self {
        Statement::Block(Block { statements })
    }
}

// Visitor pattern for AST traversal
pub trait AstVisitor<T> {
    fn visit_program(&mut self, program: &Program) -> T;
    fn visit_statement(&mut self, stmt: &Statement) -> T;
    fn visit_expression(&mut self, expr: &Expression) -> T;
}

// Pretty printer for AST debugging
pub struct AstPrinter {
    indent_level: usize,
    output_buffer: String,
}

impl AstPrinter {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            output_buffer: String::new(),
        }
    }

    pub fn print(&mut self, program: &Program) -> String {
        self.visit_program(program)
    }

    pub fn output(&self) -> &str {
        &self.output_buffer
    }

    fn indent(&self) -> String {
        "  ".repeat(self.indent_level)
    }
}

impl AstVisitor<String> for AstPrinter {
    fn visit_program(&mut self, program: &Program) -> String {
        let mut result = String::new();
        result.push_str("Program {\n");
        self.indent_level += 1;

        for stmt in &program.statements {
            result.push_str(&format!(
                "{}{}\n",
                self.indent(),
                self.visit_statement(stmt)
            ));
        }

        self.indent_level -= 1;
        result.push('}');
        self.output_buffer = result.clone();
        result
    }

    fn visit_statement(&mut self, stmt: &Statement) -> String {
        match stmt {
            Statement::VarDecl(var_decl) => {
                format!(
                    "VarDecl {{ name: {}, initializer: {} }}",
                    var_decl.name,
                    self.visit_expression(&var_decl.initializer)
                )
            }
            Statement::FuncDecl(func_decl) => {
                format!(
                    "FuncDecl {{ name: {}, params: {:?}, body: ... }}",
                    func_decl.name, func_decl.params
                )
            }
            Statement::ExprStmt(expr_stmt) => {
                format!(
                    "ExprStmt {{ {} }}",
                    self.visit_expression(&expr_stmt.expression)
                )
            }
            Statement::IfStmt(if_stmt) => {
                format!(
                    "IfStmt {{ condition: {}, then: ..., else: ... }}",
                    self.visit_expression(&if_stmt.condition)
                )
            }
            Statement::WhileStmt(while_stmt) => {
                format!(
                    "WhileStmt {{ condition: {}, body: ... }}",
                    self.visit_expression(&while_stmt.condition)
                )
            }
            Statement::ReturnStmt(return_stmt) => {
                format!("ReturnStmt {{ value: {:?} }}", return_stmt.value)
            }
            Statement::Block(_) => "Block { ... }".to_string(),
        }
    }

    fn visit_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::Binary(binary) => {
                format!(
                    "({} {} {})",
                    self.visit_expression(&binary.left),
                    binary.operator,
                    self.visit_expression(&binary.right)
                )
            }
            Expression::Unary(unary) => {
                format!(
                    "({}{})",
                    unary.operator,
                    self.visit_expression(&unary.operand)
                )
            }
            Expression::Call(call) => {
                format!(
                    "{}({})",
                    self.visit_expression(&call.callee),
                    call.arguments
                        .iter()
                        .map(|arg| self.visit_expression(arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Expression::Assignment(assignment) => {
                format!(
                    "{} = {}",
                    assignment.name,
                    self.visit_expression(&assignment.value)
                )
            }
            Expression::Variable(variable) => variable.name.clone(),
            Expression::Literal(literal) => match literal {
                LiteralExpr::Number(n) => n.to_string(),
                LiteralExpr::Boolean(b) => b.to_string(),
                LiteralExpr::String(s) => format!("\"{}\"", s),
                LiteralExpr::Array(elements) => {
                    format!(
                        "[{}]",
                        elements
                            .iter()
                            .map(|e| self.visit_expression(e))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            },
            Expression::MemberAccess(member) => {
                format!(
                    "{}.{}",
                    self.visit_expression(&member.object),
                    member.property
                )
            }
            Expression::StorageAccess(storage) => match storage {
                StorageAccessExpr::Get(key) => {
                    format!("storage.get({})", self.visit_expression(key))
                }
                StorageAccessExpr::Set(key, value) => {
                    format!(
                        "storage.set({}, {})",
                        self.visit_expression(key),
                        self.visit_expression(value)
                    )
                }
            },
            Expression::MemoryAccess(memory) => match memory {
                MemoryAccessExpr::Load(offset) => {
                    format!("memory.load({})", self.visit_expression(offset))
                }
                MemoryAccessExpr::Store(offset, value) => {
                    format!(
                        "memory.store({}, {})",
                        self.visit_expression(offset),
                        self.visit_expression(value)
                    )
                }
            },
            Expression::ArrayAccess(array_access) => {
                format!(
                    "{}[{}]",
                    self.visit_expression(&array_access.object),
                    self.visit_expression(&array_access.index)
                )
            }
            Expression::MemoryAssignment(mem_assign) => {
                format!("memory = {}", self.visit_expression(&mem_assign.value))
            }
            Expression::StorageArrayAssignment(storage_assign) => {
                format!(
                    "storage[{}] = {}",
                    self.visit_expression(&storage_assign.index),
                    self.visit_expression(&storage_assign.value)
                )
            }
        }
    }
}
