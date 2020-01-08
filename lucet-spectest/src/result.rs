use crate::error::Error;
use wabt::script::{Command, CommandKind};

pub struct SpecScriptResult {
    pass: Vec<Command>,
    skip: Vec<(Command, Error)>,
    fail: Vec<(Command, Error)>,
}

impl SpecScriptResult {
    pub fn new() -> SpecScriptResult {
        Self {
            pass: Vec::new(),
            skip: Vec::new(),
            fail: Vec::new(),
        }
    }

    pub fn pass(&mut self, command: &Command) {
        self.pass.push(command.clone())
    }

    pub fn skip(&mut self, command: &Command, reason: Error) {
        self.skip.push((command.clone(), reason))
    }

    pub fn fail(&mut self, command: &Command, reason: Error) {
        self.fail.push((command.clone(), reason))
    }

    pub fn passed(&self) -> &[Command] {
        &self.pass
    }

    pub fn skipped(&self) -> &[(Command, Error)] {
        &self.skip
    }

    pub fn failed(&self) -> &[(Command, Error)] {
        &self.fail
    }

    pub fn report(&self) {
        println!("{} passed", self.pass.len());
        if self.skip.len() > 0 {
            println!("{} skipped:", self.skip.len());
            for (ref cmd, ref err) in &self.skip {
                println!(
                    "SKIP in {}, line {}: {}",
                    command_description(&cmd.kind),
                    cmd.line,
                    err
                );
            }
        }

        if self.fail.len() > 0 {
            println!("{} failures:", self.fail.len());
            for (ref cmd, ref err) in &self.fail {
                println!(
                    "FAIL in {}, line {}: {:?}",
                    command_description(&cmd.kind),
                    cmd.line,
                    err
                );
            }
        }
    }
}

pub fn command_description(cmd: &CommandKind) -> &'static str {
    match cmd {
        CommandKind::Module { .. } => "Module",
        CommandKind::AssertReturn { .. } => "AssertReturn",
        CommandKind::AssertReturnCanonicalNan { .. } => "AssertReturnCanonicalNan",
        CommandKind::AssertReturnArithmeticNan { .. } => "AssertReturnArithmeticNan",
        CommandKind::AssertTrap { .. } => "AssertTrap",
        CommandKind::AssertInvalid { .. } => "AssertTrap",
        CommandKind::AssertMalformed { .. } => "AssertMalformed",
        CommandKind::AssertUninstantiable { .. } => "AssertUninstantiable",
        CommandKind::AssertExhaustion { .. } => "AssertExhaustion",
        CommandKind::AssertUnlinkable { .. } => "AssertUnlinkable",
        CommandKind::Register { .. } => "Register",
        CommandKind::PerformAction(_) => "PerformAction",
    }
}
