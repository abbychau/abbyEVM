use crate::compiler::ast::*;
use crate::opcodes::OpCode;
use std::collections::HashMap;
use ethereum_types::U256;

struct PendingJump {
    push_opcode_pos: usize,  // Position of the PUSH opcode
    data_start_pos: usize,   // Position where the address bytes start
    label: String,
}

pub struct CodeGenerator {
    pub bytecode: Vec<u8>,
    variables: HashMap<String, u16>, // Variable name -> stack offset
    functions: HashMap<String, u16>, // Function name -> bytecode address
    stack_depth: u16,
    next_var_slot: u16,
    jump_labels: HashMap<String, u16>, // Jump label -> address
    next_label_id: u32,
    memory_pointer: u16, // Current memory position for implicit allocation
    pending_jumps: Vec<PendingJump>, // Jump fixup information
}

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Compile error: {}", self.message)
    }
}

impl std::error::Error for CompileError {}

type CompileResult<T> = Result<T, CompileError>;

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            bytecode: Vec::new(),
            variables: HashMap::new(),
            functions: HashMap::new(),
            stack_depth: 0,
            next_var_slot: 0,
            jump_labels: HashMap::new(),
            next_label_id: 0,
            memory_pointer: 0x80, // Start at 0x80 (common EVM convention)
            pending_jumps: Vec::new(),
        }
    }
    
    pub fn compile(&mut self, program: &Program) -> CompileResult<Vec<u8>> {
        self.visit_program(program)?;
        
        // Fix up all pending jumps
        self.fixup_jumps()?;
        
        // Add STOP at the end if not already present
        if self.bytecode.is_empty() || *self.bytecode.last().unwrap() != 0x00 {
            self.emit_opcode(OpCode::STOP);
        }
        
        Ok(self.bytecode.clone())
    }
    
    fn fixup_jumps(&mut self) -> CompileResult<()> {
        for jump in &self.pending_jumps {
            if let Some(&target_addr) = self.jump_labels.get(&jump.label) {
                // Always use PUSH2 for jump addresses to avoid complications
                let high_byte = (target_addr >> 8) as u8;
                let low_byte = (target_addr & 0xFF) as u8;
                
                self.bytecode[jump.push_opcode_pos] = OpCode::PUSH2.to_byte();
                self.bytecode[jump.data_start_pos] = high_byte;
                self.bytecode[jump.data_start_pos + 1] = low_byte;
            } else {
                return Err(CompileError { 
                    message: format!("Undefined jump label: {}", jump.label) 
                });
            }
        }
        Ok(())
    }
    
    // AST visitor methods
    
    fn visit_program(&mut self, program: &Program) -> CompileResult<()> {
        // First pass: collect function declarations
        for stmt in &program.statements {
            if let Statement::FuncDecl(func_decl) = stmt {
                self.functions.insert(func_decl.name.clone(), self.bytecode.len() as u16);
            }
        }
        
        // Second pass: generate code
        for stmt in &program.statements {
            self.visit_statement(stmt)?;
        }
        
        Ok(())
    }
    
    fn visit_statement(&mut self, stmt: &Statement) -> CompileResult<()> {
        match stmt {
            Statement::VarDecl(var_decl) => self.visit_var_decl(var_decl),
            Statement::FuncDecl(func_decl) => self.visit_func_decl(func_decl),
            Statement::ExprStmt(expr_stmt) => {
                self.visit_expression(&expr_stmt.expression)?;
                // Pop the result since it's not used
                self.emit_opcode(OpCode::POP);
                self.stack_depth -= 1;
                Ok(())
            },
            Statement::IfStmt(if_stmt) => self.visit_if_stmt(if_stmt),
            Statement::WhileStmt(while_stmt) => self.visit_while_stmt(while_stmt),
            Statement::ReturnStmt(return_stmt) => self.visit_return_stmt(return_stmt),
            Statement::Block(block) => self.visit_block(block),
        }
    }
    
    fn visit_var_decl(&mut self, var_decl: &VarDecl) -> CompileResult<()> {
        // Generate code for the initializer
        self.visit_expression(&var_decl.initializer)?;
        
        // Store the variable in the next available slot
        let slot = self.next_var_slot;
        self.variables.insert(var_decl.name.clone(), slot);
        self.next_var_slot += 1;
        
        // Duplicate the value on stack so we can store it
        self.emit_opcode(OpCode::DUP1);
        self.stack_depth += 1;
        
        // Store in storage (for persistent variables)
        self.emit_push_u256(U256::from(slot));
        self.stack_depth += 1;
        self.emit_opcode(OpCode::SSTORE);
        self.stack_depth -= 2;
        
        Ok(())
    }
    
    fn visit_func_decl(&mut self, func_decl: &FuncDecl) -> CompileResult<()> {
        // Function declarations are handled in the first pass
        // Here we generate the actual function body
        let _function_start = self.bytecode.len();
        
        // Create a new scope for function parameters
        let saved_vars = self.variables.clone();
        let saved_next_slot = self.next_var_slot;
        
        // Add parameters as variables
        for (i, param) in func_decl.params.iter().enumerate() {
            self.variables.insert(param.clone(), i as u16);
        }
        
        // Generate function body
        self.visit_block(&func_decl.body)?;
        
        // If no explicit return, add default return 0
        self.emit_push_u256(U256::zero());
        self.emit_opcode(OpCode::RETURN);
        
        // Restore previous scope
        self.variables = saved_vars;
        self.next_var_slot = saved_next_slot;
        
        Ok(())
    }
    
    fn visit_if_stmt(&mut self, if_stmt: &IfStmt) -> CompileResult<()> {
        // Generate condition
        self.visit_expression(&if_stmt.condition)?;
        
        // Generate unique labels
        let else_label = self.generate_label("else");
        let end_label = self.generate_label("end_if");
        
        // Jump to else if condition is false (0)
        self.emit_opcode(OpCode::ISZERO); // Invert condition
        self.emit_jump_if(&else_label);
        self.stack_depth -= 1;
        
        // Generate then branch
        self.visit_statement(&if_stmt.then_branch)?;
        
        // Jump to end
        self.emit_jump(&end_label);
        
        // Else label
        self.place_label(&else_label);
        
        // Generate else branch if present
        if let Some(else_branch) = &if_stmt.else_branch {
            self.visit_statement(else_branch)?;
        }
        
        // End label
        self.place_label(&end_label);
        
        Ok(())
    }
    
    fn visit_while_stmt(&mut self, while_stmt: &WhileStmt) -> CompileResult<()> {
        let loop_start = self.generate_label("loop_start");
        let loop_end = self.generate_label("loop_end");
        
        // Loop start
        self.place_label(&loop_start);
        
        // Generate condition
        self.visit_expression(&while_stmt.condition)?;
        
        // Jump to end if condition is false
        self.emit_opcode(OpCode::ISZERO);
        self.emit_jump_if(&loop_end);
        self.stack_depth -= 1;
        
        // Generate body
        self.visit_statement(&while_stmt.body)?;
        
        // Jump back to start
        self.emit_jump(&loop_start);
        
        // End label
        self.place_label(&loop_end);
        
        Ok(())
    }
    
    fn visit_return_stmt(&mut self, return_stmt: &ReturnStmt) -> CompileResult<()> {
        if let Some(value) = &return_stmt.value {
            self.visit_expression(value)?;
        } else {
            self.emit_push_u256(U256::zero());
            self.stack_depth += 1;
        }
        
        // RETURN expects offset and size on stack
        // For now, just return the value directly (simplified)
        self.emit_push_u256(U256::from(32)); // size
        self.emit_push_u256(U256::zero()); // offset
        self.emit_opcode(OpCode::MSTORE); // Store return value in memory (consumes 3: value, offset, size)
        if self.stack_depth >= 3 {
            self.stack_depth -= 3;
        } else {
            self.stack_depth = 0;
        }
        
        self.emit_push_u256(U256::from(32)); // size
        self.emit_push_u256(U256::zero()); // offset
        self.emit_opcode(OpCode::RETURN);
        if self.stack_depth >= 2 {
            self.stack_depth -= 2;
        } else {
            self.stack_depth = 0;
        }
        
        Ok(())
    }
    
    fn visit_block(&mut self, block: &Block) -> CompileResult<()> {
        for stmt in &block.statements {
            self.visit_statement(stmt)?;
        }
        Ok(())
    }
    
    pub fn visit_expression(&mut self, expr: &Expression) -> CompileResult<()> {
        match expr {
            Expression::Binary(binary) => self.visit_binary_expr(binary),
            Expression::Unary(unary) => self.visit_unary_expr(unary),
            Expression::Call(call) => self.visit_call_expr(call),
            Expression::Assignment(assignment) => self.visit_assignment_expr(assignment),
            Expression::Variable(variable) => self.visit_variable_expr(variable),
            Expression::Literal(literal) => self.visit_literal_expr(literal),
            Expression::MemberAccess(member) => self.visit_member_access_expr(member),
            Expression::StorageAccess(storage) => self.visit_storage_access_expr(storage),
            Expression::MemoryAccess(memory) => self.visit_memory_access_expr(memory),
            Expression::ArrayAccess(array_access) => self.visit_array_access_expr(array_access),
            Expression::MemoryAssignment(mem_assign) => self.visit_memory_assignment_expr(mem_assign),
            Expression::StorageArrayAssignment(storage_assign) => self.visit_storage_array_assignment_expr(storage_assign),
        }
    }
    
    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> CompileResult<()> {
        // Generate left operand
        self.visit_expression(&binary.left)?;
        
        // Generate right operand
        self.visit_expression(&binary.right)?;
        
        // Generate operator
        match binary.operator {
            BinaryOperator::Add => self.emit_opcode(OpCode::ADD),
            BinaryOperator::Subtract => self.emit_opcode(OpCode::SUB),
            BinaryOperator::Multiply => self.emit_opcode(OpCode::MUL),
            BinaryOperator::Divide => self.emit_opcode(OpCode::DIV),
            BinaryOperator::Modulo => self.emit_opcode(OpCode::MOD),
            BinaryOperator::Equal => self.emit_opcode(OpCode::EQ),
            BinaryOperator::NotEqual => {
                self.emit_opcode(OpCode::EQ);
                self.emit_opcode(OpCode::ISZERO); // Invert result
            },
            BinaryOperator::Greater => self.emit_opcode(OpCode::GT),
            BinaryOperator::GreaterEqual => {
                self.emit_opcode(OpCode::LT);
                self.emit_opcode(OpCode::ISZERO); // Invert result of LT
            },
            BinaryOperator::Less => self.emit_opcode(OpCode::LT),
            BinaryOperator::LessEqual => {
                self.emit_opcode(OpCode::GT);
                self.emit_opcode(OpCode::ISZERO); // Invert result of GT
            },
            BinaryOperator::And => {
                // Logical AND: both operands must be non-zero
                self.emit_opcode(OpCode::AND);
                self.emit_push_u256(U256::zero());
                self.stack_depth += 1;
                self.emit_opcode(OpCode::GT); // Result > 0
                self.stack_depth -= 1;
            },
            BinaryOperator::Or => {
                // Logical OR: at least one operand must be non-zero
                self.emit_opcode(OpCode::OR);
                self.emit_push_u256(U256::zero());
                self.stack_depth += 1;
                self.emit_opcode(OpCode::GT); // Result > 0
                self.stack_depth -= 1;
            },
        }
        
        self.stack_depth -= 1; // Binary ops consume 2, produce 1
        
        Ok(())
    }
    
    fn visit_unary_expr(&mut self, unary: &UnaryExpr) -> CompileResult<()> {
        self.visit_expression(&unary.operand)?;
        
        match unary.operator {
            UnaryOperator::Minus => {
                // Negate by subtracting from 0
                self.emit_push_u256(U256::zero());
                self.stack_depth += 1;
                self.emit_opcode(OpCode::SUB); // 0 - operand
                self.stack_depth -= 1;
            },
            UnaryOperator::Not => {
                self.emit_opcode(OpCode::ISZERO);
            },
        }
        
        Ok(())
    }
    
    fn visit_call_expr(&mut self, call: &CallExpr) -> CompileResult<()> {
        // Handle different types of function calls
        match &*call.callee {
            Expression::Variable(var) => {
                // Simple function calls like keccak256()
                match var.name.as_str() {
                    "keccak256" => {
                        if call.arguments.len() != 1 {
                            return Err(CompileError {
                                message: "keccak256 expects exactly 1 argument".to_string(),
                            });
                        }
                        
                        // For simplicity, we'll just hash a constant for now
                        // In a real implementation, we'd handle dynamic input
                        self.emit_push_u256(U256::from(32)); // size
                        self.emit_push_u256(U256::zero()); // offset
                        self.emit_opcode(OpCode::SHA3);
                        self.stack_depth += 1;
                    },
                    "println" => {
                        // Legacy support for println - treat as console.log
                        return Err(CompileError {
                            message: "println is not a JavaScript function. Use console.log, console.warn, or console.error instead".to_string(),
                        });
                    },
                    _ => {
                        return Err(CompileError {
                            message: format!("Unknown function: {}", var.name),
                        });
                    },
                }
            },
            Expression::MemberAccess(member) => {
                // Handle member access calls like console.log(), console.warn(), etc.
                if let Expression::Variable(obj) = &*member.object {
                    if obj.name == "console" {
                        match member.property.as_str() {
                            "log" | "warn" | "error" => {
                                // Handle console methods with flexible argument types
                                if call.arguments.is_empty() {
                                    // No arguments - output empty string
                                    let offset = self.memory_pointer;
                                    self.emit_push_u256(U256::from(0)); // size = 0
                                    self.emit_push_u256(U256::from(offset)); // offset
                                } else if call.arguments.len() == 2 {
                                    // Special case for two arguments (most common case)
                                    let arg1 = &call.arguments[0];
                                    let arg2 = &call.arguments[1];
                                    
                                    // Process first argument (usually a string)
                                    match arg1 {
                                        Expression::Literal(LiteralExpr::String(s)) => {
                                            let start_offset = self.memory_pointer;
                                            
                                            // Store string
                                            for byte in s.bytes() {
                                                self.emit_push_u256(U256::from(byte));
                                                self.emit_push_u256(U256::from(self.memory_pointer));
                                                self.emit_opcode(OpCode::MSTORE8);
                                                self.stack_depth += 2;
                                                self.stack_depth -= 2;
                                                self.memory_pointer += 1;
                                            }
                                            
                                            // Add space
                                            self.emit_push_u256(U256::from(32)); // ASCII space
                                            self.emit_push_u256(U256::from(self.memory_pointer));
                                            self.emit_opcode(OpCode::MSTORE8);
                                            self.stack_depth += 2;
                                            self.stack_depth -= 2;
                                            self.memory_pointer += 1;
                                            
                                            // Process second argument (usually a variable)
                                            {
                                                // Variable or expression
                                                self.visit_expression(arg2)?;
                                                
                                                // Convert to ASCII and store (single digit)
                                                self.emit_push_u256(U256::from(48)); // ASCII '0'
                                                self.stack_depth += 1;
                                                self.emit_opcode(OpCode::ADD);
                                                self.stack_depth -= 1;
                                                
                                                self.emit_push_u256(U256::from(self.memory_pointer));
                                                self.stack_depth += 1;
                                                self.emit_opcode(OpCode::MSTORE8);
                                                self.stack_depth -= 2;
                                                self.memory_pointer += 1;
                                            }
                                            
                                            let total_length = self.memory_pointer - start_offset;
                                            self.emit_push_u256(U256::from(total_length));
                                            self.emit_push_u256(U256::from(start_offset));
                                            self.stack_depth += 2;
                                        },
                                        _ => {
                                            // Fallback: just process first argument
                                            self.visit_expression(arg1)?;
                                            self.emit_opcode(OpCode::SWAP1);
                                        }
                                    }
                                } else {
                                    // For now, just handle first argument if more than 2
                                    let arg = &call.arguments[0];
                                    
                                    match arg {
                                        Expression::Literal(LiteralExpr::String(_)) => {
                                            self.visit_expression(arg)?;
                                            self.emit_opcode(OpCode::SWAP1);
                                        },
                                        Expression::Literal(LiteralExpr::Number(n)) => {
                                            let number_str = n.to_string();
                                            let offset = self.memory_pointer;
                                            
                                            for (i, byte) in number_str.bytes().enumerate() {
                                                self.emit_push_u256(U256::from(byte));
                                                self.emit_push_u256(U256::from(offset + i as u16));
                                                self.emit_opcode(OpCode::MSTORE8);
                                                self.stack_depth += 2;
                                                self.stack_depth -= 2;
                                            }
                                            
                                            self.emit_push_u256(U256::from(number_str.len()));
                                            self.emit_push_u256(U256::from(offset));
                                            self.stack_depth += 2;
                                            self.memory_pointer += number_str.len() as u16;
                                        },
                                        _ => {
                                            self.visit_expression(arg)?;
                                            let offset = self.memory_pointer;
                                            self.emit_number_to_string_conversion(offset)?;
                                        }
                                    }
                                }
                                
                                // Emit the appropriate LOG opcode
                                match member.property.as_str() {
                                    "log" => {
                                        // Standard console.log - use LOG0
                                        // Stack: [length, offset] which is what LOG0 expects
                                        self.emit_opcode(OpCode::LOG0);
                                    },
                                    "warn" => {
                                        // Console warning - use LOG1 with a warning topic
                                        self.emit_push_u256(U256::from(1)); // Warning topic
                                        self.stack_depth += 1;
                                        // Stack: [length, offset, topic1]
                                        // LOG1 expects: [offset, size, topic1]
                                        // We need to rearrange: [topic1, size, offset] -> [offset, size, topic1]
                                        self.emit_opcode(OpCode::SWAP2); // [topic1, offset, length]
                                        self.emit_opcode(OpCode::SWAP1); // [topic1, length, offset]
                                        // Actually LOG1 in EVM expects: stack with topic1 on top, then size, then offset
                                        // So current stack [topic1, length, offset] is correct
                                        self.emit_opcode(OpCode::LOG1);
                                        self.stack_depth -= 1; // LOG1 consumes the topic
                                    },
                                    "error" => {
                                        // Console error - use LOG1 with an error topic  
                                        self.emit_push_u256(U256::from(2)); // Error topic
                                        self.stack_depth += 1;
                                        // Stack: [length, offset, topic2]
                                        self.emit_opcode(OpCode::SWAP2); // [topic2, offset, length]
                                        self.emit_opcode(OpCode::SWAP1); // [topic2, length, offset]
                                        self.emit_opcode(OpCode::LOG1);
                                        self.stack_depth -= 1; // LOG1 consumes the topic
                                    },
                                    _ => unreachable!(), // We already matched these above
                                }
                                
                                self.stack_depth -= 2; // All LOG opcodes consume offset and size
                                
                                // Push a dummy return value so expression statement can pop it
                                self.emit_push_u256(U256::zero());
                                self.stack_depth += 1;
                            },
                            _ => {
                                return Err(CompileError {
                                    message: format!("Unknown console method: {}", member.property),
                                });
                            },
                        }
                    } else {
                        return Err(CompileError {
                            message: format!("Member access not supported for object: {}", obj.name),
                        });
                    }
                } else {
                    return Err(CompileError {
                        message: "Complex member access not yet supported".to_string(),
                    });
                }
            },
            _ => {
                return Err(CompileError {
                    message: "Complex function calls not yet supported".to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    fn visit_assignment_expr(&mut self, assignment: &AssignmentExpr) -> CompileResult<()> {
        // Generate value
        self.visit_expression(&assignment.value)?;
        
        // Get variable slot
        let slot = *self.variables.get(&assignment.name)
            .ok_or_else(|| CompileError {
                message: format!("Undefined variable: {}", assignment.name),
            })?;
        
        // Duplicate value for return
        self.emit_opcode(OpCode::DUP1);
        self.stack_depth += 1;
        
        // Store value
        self.emit_push_u256(U256::from(slot));
        self.stack_depth += 1;
        self.emit_opcode(OpCode::SSTORE);
        self.stack_depth -= 2;
        
        Ok(())
    }
    
    fn visit_variable_expr(&mut self, variable: &VariableExpr) -> CompileResult<()> {
        match variable.name.as_str() {
            "memory" => {
                // Reading from memory without index -> load from current memory pointer - 32
                let read_offset = self.memory_pointer.saturating_sub(32);
                self.emit_push_u256(U256::from(read_offset));
                self.stack_depth += 1;
                self.emit_opcode(OpCode::MLOAD);
            },
            _ => {
                // Regular variable access
                let slot = *self.variables.get(&variable.name)
                    .ok_or_else(|| CompileError {
                        message: format!("Undefined variable: {}", variable.name),
                    })?;
                
                // Load variable from storage
                self.emit_push_u256(U256::from(slot));
                self.stack_depth += 1;
                self.emit_opcode(OpCode::SLOAD);
            }
        }
        
        Ok(())
    }
    
    fn visit_member_access_expr(&mut self, member: &MemberAccessExpr) -> CompileResult<()> {
        // Handle member access expressions like console.log, console.warn, etc.
        // For now, we don't actually emit code for member access itself - 
        // it will be handled by the CallExpr that uses this as a callee
        // This is a placeholder that returns an error if used outside of calls
        Err(CompileError {
            message: "Member access expressions are only supported in function calls".to_string(),
        })
    }
    
    fn visit_literal_expr(&mut self, literal: &LiteralExpr) -> CompileResult<()> {
        match literal {
            LiteralExpr::Number(n) => {
                self.emit_push_u256(U256::from(*n));
                self.stack_depth += 1;
            },
            LiteralExpr::Boolean(b) => {
                self.emit_push_u256(if *b { U256::one() } else { U256::zero() });
                self.stack_depth += 1;
            },
            LiteralExpr::String(s) => {
                // Store string in memory and push memory offset and length
                let offset = self.memory_pointer;
                let len = s.len();
                
                // Store each byte of the string in memory
                for (i, byte) in s.bytes().enumerate() {
                    self.emit_push_u256(U256::from(byte)); // value (first)
                    self.emit_push_u256(U256::from(offset + i as u16)); // offset (second)
                    self.emit_opcode(OpCode::MSTORE8); // MSTORE8 pops offset, then value
                    self.stack_depth += 2; // We pushed 2 values
                    self.stack_depth -= 2; // MSTORE8 consumes 2 values
                }
                
                // Push offset and length on stack for println to use
                self.emit_push_u256(U256::from(offset));
                self.emit_push_u256(U256::from(len));
                self.stack_depth += 2;
                
                // Update memory pointer for next allocation
                self.memory_pointer += len as u16;
            },
            LiteralExpr::Array(elements) => {
                // For empty arrays, just push a special marker (0)
                if elements.is_empty() {
                    self.emit_push_u256(U256::zero());
                    self.stack_depth += 1;
                } else {
                    // For non-empty arrays, we'd need to implement array handling
                    // For now, just push the number of elements
                    self.emit_push_u256(U256::from(elements.len()));
                    self.stack_depth += 1;
                }
            },
        }
        Ok(())
    }
    
    fn visit_storage_access_expr(&mut self, storage: &StorageAccessExpr) -> CompileResult<()> {
        match storage {
            StorageAccessExpr::Get(key) => {
                self.visit_expression(key)?;
                self.emit_opcode(OpCode::SLOAD);
            },
            StorageAccessExpr::Set(key, value) => {
                self.visit_expression(value)?;
                self.visit_expression(key)?;
                self.emit_opcode(OpCode::SSTORE);
                self.stack_depth -= 2;
                // Push the value back for return
                self.visit_expression(value)?;
            },
        }
        Ok(())
    }
    
    fn visit_memory_access_expr(&mut self, memory: &MemoryAccessExpr) -> CompileResult<()> {
        match memory {
            MemoryAccessExpr::Load(offset) => {
                self.visit_expression(offset)?;
                self.emit_opcode(OpCode::MLOAD);
            },
            MemoryAccessExpr::Store(offset, value) => {
                self.visit_expression(value)?;
                self.visit_expression(offset)?;
                self.emit_opcode(OpCode::MSTORE);
                self.stack_depth -= 2;
                // Push the value back for return
                self.visit_expression(value)?;
            },
        }
        Ok(())
    }
    
    fn visit_array_access_expr(&mut self, array_access: &ArrayAccessExpr) -> CompileResult<()> {
        // Handle array access like storage[key] or memory[offset]
        if let Expression::Variable(var) = &*array_access.object {
            match var.name.as_str() {
                "storage" => {
                    // storage[key] -> SLOAD
                    self.visit_expression(&array_access.index)?;
                    self.emit_opcode(OpCode::SLOAD);
                },
                "memory" => {
                    // memory[offset] -> MLOAD
                    self.visit_expression(&array_access.index)?;
                    self.emit_opcode(OpCode::MLOAD);
                },
                _ => {
                    return Err(CompileError {
                        message: format!("Array access not supported for '{}'", var.name),
                    });
                }
            }
        } else {
            return Err(CompileError {
                message: "Complex array access not yet supported".to_string(),
            });
        }
        Ok(())
    }
    
    fn visit_memory_assignment_expr(&mut self, mem_assign: &MemoryAssignmentExpr) -> CompileResult<()> {
        // memory = value -> store at current memory pointer and increment
        self.visit_expression(&mem_assign.value)?;
        
        // Duplicate value for return
        self.emit_opcode(OpCode::DUP1);
        self.stack_depth += 1;
        
        // Store at current memory pointer
        self.emit_push_u256(U256::from(self.memory_pointer));
        self.stack_depth += 1;
        self.emit_opcode(OpCode::MSTORE);
        self.stack_depth -= 2;
        
        // Increment memory pointer for next allocation
        self.memory_pointer += 32; // EVM words are 32 bytes
        
        Ok(())
    }
    
    fn visit_storage_array_assignment_expr(&mut self, storage_assign: &StorageArrayAssignmentExpr) -> CompileResult<()> {
        // storage[key] = value -> SSTORE
        self.visit_expression(&storage_assign.value)?;
        self.visit_expression(&storage_assign.index)?;
        self.emit_opcode(OpCode::SSTORE);
        self.stack_depth -= 2;
        
        // Push the value back for return
        self.visit_expression(&storage_assign.value)?;
        
        Ok(())
    }
    
    // Code emission helpers
    
    pub fn emit_opcode(&mut self, opcode: OpCode) {
        self.bytecode.push(opcode.to_byte());
    }
    
    fn emit_push_u256(&mut self, value: U256) {
        let bytes = self.u256_to_minimal_bytes(value);
        let push_opcode = match bytes.len() {
            1 => OpCode::PUSH1,
            2 => OpCode::PUSH2,
            3 => OpCode::PUSH3,
            4 => OpCode::PUSH4,
            _ => OpCode::PUSH32, // Use PUSH32 for larger values
        };
        
        self.bytecode.push(push_opcode.to_byte());
        if bytes.len() <= 32 {
            self.bytecode.extend(bytes);
        } else {
            // For values larger than 32 bytes, pad to 32 bytes
            let mut padded = vec![0u8; 32];
            let start = 32 - bytes.len().min(32);
            padded[start..].copy_from_slice(&bytes[..bytes.len().min(32)]);
            self.bytecode.extend(padded);
        }
    }
    
    fn u256_to_minimal_bytes(&self, value: U256) -> Vec<u8> {
        let mut bytes = [0u8; 32];
        value.to_big_endian(&mut bytes);
        
        // Find first non-zero byte
        let start = bytes.iter().position(|&b| b != 0).unwrap_or(31);
        bytes[start..].to_vec()
    }
    
    fn generate_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.next_label_id);
        self.next_label_id += 1;
        label
    }
    
    fn place_label(&mut self, label: &str) {
        self.jump_labels.insert(label.to_string(), self.bytecode.len() as u16);
        self.emit_opcode(OpCode::JUMPDEST);
    }
    
    fn emit_jump(&mut self, label: &str) {
        // Reserve space for PUSH2 instruction (3 bytes total: opcode + 2 data bytes)
        let push_opcode_pos = self.bytecode.len();
        let data_start_pos = push_opcode_pos + 1;
        
        self.pending_jumps.push(PendingJump {
            push_opcode_pos,
            data_start_pos,
            label: label.to_string(),
        });
        
        // Reserve space: PUSH2 opcode + 2 placeholder bytes
        self.bytecode.push(0x00); // Placeholder for PUSH2 opcode
        self.bytecode.push(0x00); // Placeholder for high byte
        self.bytecode.push(0x00); // Placeholder for low byte
        
        self.stack_depth += 1;
        self.emit_opcode(OpCode::JUMP);
        self.stack_depth -= 1;
    }
    
    fn emit_jump_if(&mut self, label: &str) {
        // Reserve space for PUSH2 instruction (3 bytes total: opcode + 2 data bytes)
        let push_opcode_pos = self.bytecode.len();
        let data_start_pos = push_opcode_pos + 1;
        
        self.pending_jumps.push(PendingJump {
            push_opcode_pos,
            data_start_pos,
            label: label.to_string(),
        });
        
        // Reserve space: PUSH2 opcode + 2 placeholder bytes
        self.bytecode.push(0x00); // Placeholder for PUSH2 opcode
        self.bytecode.push(0x00); // Placeholder for high byte
        self.bytecode.push(0x00); // Placeholder for low byte
        
        self.stack_depth += 1;
        self.emit_opcode(OpCode::JUMPI);
        self.stack_depth -= 2; // JUMPI consumes two stack items (condition and address)
    }
    
    fn emit_number_to_string_conversion(&mut self, offset: u16) -> CompileResult<()> {
        // Super simple version: only handle single digits properly for now
        // Stack has: [number]
        
        let base_offset = offset;
        
        // For numbers 0-9, just convert to ASCII and store
        self.emit_push_u256(U256::from(48)); // ASCII '0'
        self.stack_depth += 1;
        self.emit_opcode(OpCode::ADD); // [ascii_number]
        self.stack_depth -= 1;
        
        // Store the single digit
        self.emit_push_u256(U256::from(base_offset));
        self.stack_depth += 1;
        self.emit_opcode(OpCode::MSTORE8); // []
        self.stack_depth -= 2;
        
        // Return length=1, offset=base_offset
        self.emit_push_u256(U256::from(1)); // length
        self.emit_push_u256(U256::from(base_offset)); // offset
        self.stack_depth += 2;
        
        self.memory_pointer += 1;
        
        Ok(())
    }
}

// Extension trait to convert OpCode to byte
trait OpCodeExt {
    fn to_byte(&self) -> u8;
}

impl OpCodeExt for OpCode {
    fn to_byte(&self) -> u8 {
        match self {
            OpCode::STOP => 0x00,
            OpCode::ADD => 0x01,
            OpCode::MUL => 0x02,
            OpCode::SUB => 0x03,
            OpCode::DIV => 0x04,
            OpCode::MOD => 0x06,
            OpCode::EXP => 0x0a,
            OpCode::LT => 0x10,
            OpCode::GT => 0x11,
            OpCode::EQ => 0x14,
            OpCode::ISZERO => 0x15,
            OpCode::AND => 0x16,
            OpCode::OR => 0x17,
            OpCode::XOR => 0x18,
            OpCode::NOT => 0x19,
            OpCode::SHA3 => 0x20,
            OpCode::POP => 0x50,
            OpCode::MLOAD => 0x51,
            OpCode::MSTORE => 0x52,
            OpCode::MSTORE8 => 0x53,
            OpCode::SLOAD => 0x54,
            OpCode::SSTORE => 0x55,
            OpCode::JUMP => 0x56,
            OpCode::JUMPI => 0x57,
            OpCode::JUMPDEST => 0x5b,
            OpCode::PUSH1 => 0x60,
            OpCode::PUSH2 => 0x61,
            OpCode::PUSH3 => 0x62,
            OpCode::PUSH4 => 0x63,
            OpCode::PUSH32 => 0x7f,
            OpCode::DUP1 => 0x80,
            OpCode::DUP2 => 0x81,
            OpCode::DUP3 => 0x82,
            OpCode::SWAP1 => 0x90,
            OpCode::SWAP2 => 0x91,
            OpCode::LOG0 => 0xa0,
            OpCode::LOG1 => 0xa1,
            OpCode::RETURN => 0xf3,
            _ => 0xfe, // INVALID for unimplemented opcodes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;
    use crate::compiler::parser::Parser;

    fn compile_expression(input: &str) -> CompileResult<Vec<u8>> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.expression().unwrap();
        
        let mut generator = CodeGenerator::new();
        generator.visit_expression(&expr)?;
        Ok(generator.bytecode)
    }

    #[test]
    fn test_simple_literal() {
        let bytecode = compile_expression("42").unwrap();
        // Should generate PUSH1 42
        assert_eq!(bytecode, vec![0x60, 42]);
    }

    #[test]
    fn test_simple_addition() {
        let bytecode = compile_expression("1 + 2").unwrap();
        // Should generate: PUSH1 1, PUSH1 2, ADD
        assert_eq!(bytecode, vec![0x60, 1, 0x60, 2, 0x01]);
    }

    #[test]
    fn test_complex_expression() {
        let bytecode = compile_expression("1 + 2 * 3").unwrap();
        // Should respect precedence: PUSH1 1, PUSH1 2, PUSH1 3, MUL, ADD
        assert_eq!(bytecode, vec![0x60, 1, 0x60, 2, 0x60, 3, 0x02, 0x01]);
    }
}
