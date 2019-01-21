use cranelift_codegen::ir::{self, Value};

#[derive(Debug)]
pub struct TranslationState {
    /// WebAssembly stack
    pub stack: Vec<Value>,
    /// WebAssembly control structures
    pub control_stack: Vec<ControlStackFrame>,
    pub reachable: bool,
}

impl TranslationState {
    /// Create a translation state for compiling a function with a given signature.
    /// The exit block is the last block in the function and contains the return instruction.
    pub fn new(sig: &ir::Signature, exit_block: ir::Ebb) -> Self {
        let mut state = Self::empty();
        state.push_control_frame(
            ControlVariant::_block(),
            exit_block,
            sig.returns
                .iter()
                .filter(|arg| arg.purpose == ir::ArgumentPurpose::Normal)
                .count(),
        );
        state
    }

    fn empty() -> Self {
        Self {
            stack: Vec::new(),
            control_stack: Vec::new(),
            reachable: true,
        }
    }

    /// Push a value
    pub fn push1(&mut self, val: Value) {
        self.stack.push(val)
    }

    /// Push multiple values
    pub fn pushn(&mut self, vals: &[Value]) {
        self.stack.extend_from_slice(vals)
    }

    /// Pop one value
    pub fn pop1(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    /// Peek at value on top of stack
    pub fn peek1(&mut self) -> Value {
        *self.stack.last().unwrap()
    }

    /// Pop two values, return in order they were pushed
    pub fn pop2(&mut self) -> (Value, Value) {
        let v2 = self.stack.pop().unwrap();
        let v1 = self.stack.pop().unwrap();
        (v1, v2)
    }

    /// Pop three values, return in order they were pushed
    pub fn pop3(&mut self) -> (Value, Value, Value) {
        let v3 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        let v1 = self.stack.pop().unwrap();
        (v1, v2, v3)
    }

    /// Drop the top `n` values on the stack.
    /// Use `peekn` to look at them before dropping
    pub fn dropn(&mut self, n: usize) {
        let new_len = self.stack.len() - n;
        self.stack.truncate(new_len);
    }

    /// Peek at top `n` values in order they were pushed
    pub fn peekn(&self, n: usize) -> &[Value] {
        &self.stack[self.stack.len() - n..]
    }

    pub fn push_control_frame(
        &mut self,
        variant: ControlVariant,
        following_code: ir::Ebb,
        num_return_values: usize,
    ) {
        let frame = ControlStackFrame {
            variant: variant,
            destination: following_code,
            original_stack_size: self.stack.len(),
            num_return_values: num_return_values,
        };
        self.control_stack.push(frame);
    }
}

#[derive(Debug)]
pub struct ControlStackFrame {
    pub variant: ControlVariant,
    pub destination: ir::Ebb,
    pub num_return_values: usize,
    pub original_stack_size: usize,
}

impl ControlStackFrame {
    pub fn following_code(&self) -> ir::Ebb {
        self.destination
    }
    pub fn br_destination(&self) -> ir::Ebb {
        match self.variant {
            ControlVariant::If { .. } | ControlVariant::Block { .. } => self.destination,
            ControlVariant::Loop { body } => body,
        }
    }
    pub fn is_loop(&self) -> bool {
        match self.variant {
            ControlVariant::Loop { .. } => true,
            _ => false,
        }
    }
    pub fn exit_is_branched_to(&self) -> bool {
        match self.variant {
            ControlVariant::If {
                exit_is_branched_to,
                ..
            }
            | ControlVariant::Block {
                exit_is_branched_to,
                ..
            } => exit_is_branched_to,
            ControlVariant::Loop { .. } => false,
        }
    }

    pub fn set_branched_to_exit(&mut self) {
        match self.variant {
            ControlVariant::If {
                ref mut exit_is_branched_to,
                ..
            }
            | ControlVariant::Block {
                ref mut exit_is_branched_to,
                ..
            } => *exit_is_branched_to = true,
            ControlVariant::Loop { .. } => {}
        }
    }
}

#[derive(Debug)]
/// Use the constructor methods defined in the impl to preserve
/// invariants. The methods have leading underscores so they do not
/// collide with rust keywords.
pub enum ControlVariant {
    If {
        branch_inst: ir::Inst,
        exit_is_branched_to: bool,
        reachable_from_top: bool,
    },
    Block {
        exit_is_branched_to: bool,
    },
    Loop {
        body: ir::Ebb,
    },
}

impl ControlVariant {
    /// Constructor for the If variant. exit_is_branched_to must be false until it is set
    /// explicitly.
    pub fn _if(branch_inst: ir::Inst, reachable_from_top: bool) -> ControlVariant {
        ControlVariant::If {
            branch_inst: branch_inst,
            exit_is_branched_to: false,
            reachable_from_top: reachable_from_top,
        }
    }
    /// Constructor for the Block variant. exit_is_branched_to must be false until it is set
    /// explicitly.
    pub fn _block() -> ControlVariant {
        ControlVariant::Block {
            exit_is_branched_to: false,
        }
    }
    /// Constructor for the Loop variant. Doesn't have the exit_is_branched_to but I didn't want to
    /// leave an odd variant without a constructor.
    pub fn _loop(body: ir::Ebb) -> ControlVariant {
        ControlVariant::Loop { body: body }
    }
}
