#![deny(bare_trait_objects)]

pub mod error;
pub mod script;

pub use crate::error::Error;
pub use crate::result::{command_description, SpecScriptResult};

mod bindings;
mod result;

use crate::script::{ScriptEnv, ScriptError};
use lucet_runtime::{Error as RuntimeError, TrapCode, UntypedRetVal, Val};
use std::fs;
use std::path::PathBuf;
use wabt::script::{Action, CommandKind, ScriptParser, Value};

pub fn run_spec_test(spec_path: &PathBuf) -> Result<SpecScriptResult, Error> {
    let wast = fs::read_to_string(spec_path).map_err(|e| Error::ReadError(e))?;
    let mut parser: ScriptParser =
        ScriptParser::from_str(&wast).map_err(|e| Error::ParseError(e))?;

    let mut script = ScriptEnv::new();
    let mut res = SpecScriptResult::new();

    while let Some(ref cmd) = parser.next()? {
        match step(&mut script, &cmd.kind) {
            Ok(()) => res.pass(cmd),
            Err(e) => match e {
                Error::UnsupportedCommand(_) | Error::UnsupportedLucetc => {
                    println!("skipped unsupported command");
                    res.skip(cmd, e)
                }
                _ => {
                    println!("command failed");
                    res.fail(cmd, e)
                }
            },
        }
    }

    Ok(res)
}

fn unexpected_failure(e: ScriptError) -> Error {
    if e.unsupported() {
        Error::UnsupportedLucetc
    } else {
        Error::UnexpectedFailure(String::new())
    }
}

fn step(script: &mut ScriptEnv, cmd: &CommandKind) -> Result<(), Error> {
    match cmd {
        CommandKind::Module {
            ref module,
            ref name,
        } => {
            println!("module {:?}", name);
            let module = module.clone().into_vec();
            script
                .instantiate(&module, name)
                .map_err(unexpected_failure)?;
            Ok(())
        }

        CommandKind::AssertInvalid { ref module, .. } => {
            println!("assert_invalid");
            let module = module.clone().into_vec();
            match script.instantiate(&module, &None) {
                Err(ScriptError::GenerationError(_, _)) => Ok(()),
                Ok(_) => {
                    script.delete_last();
                    Err(Error::UnexpectedSuccess)?
                }
                Err(e) => Err(unexpected_failure(e))?,
            }
        }

        CommandKind::AssertMalformed { ref module, .. } => {
            println!("assert_malformed");
            let module = module.clone().into_vec();
            match script.instantiate(&module, &None) {
                Err(ScriptError::GenerationError(_, _)) => Ok(()),
                Ok(_) => Err(Error::UnexpectedSuccess)?,
                Err(e) => Err(unexpected_failure(e))?,
            }
        }

        CommandKind::AssertUninstantiable { module, .. } => {
            println!("assert_uninstantiable");
            let module = module.clone().into_vec();
            match script.instantiate(&module, &None) {
                Err(ScriptError::RuntimeError(_, _)) => Ok(()),
                Ok(_) => Err(Error::UnexpectedSuccess)?,
                Err(e) => Err(unexpected_failure(e))?,
            }
        }

        CommandKind::AssertUnlinkable { module, .. } => {
            println!("assert_unlinkable");
            let module = module.clone().into_vec();
            match script.instantiate(&module, &None) {
                Err(ScriptError::GenerationError(_, _)) => Ok(()),
                Ok(_) => Err(Error::UnexpectedSuccess)?,
                Err(e) => Err(unexpected_failure(e))?,
            }
        }

        CommandKind::Register {
            ref name,
            ref as_name,
        } => {
            println!("register {:?} {}", name, as_name);
            script.register(name, as_name).map_err(unexpected_failure)?;
            Ok(())
        }

        CommandKind::PerformAction(ref action) => match action {
            Action::Invoke {
                ref module,
                ref field,
                ref args,
            } => {
                println!("invoke {:?} {} {:?}", module, field, args);
                let args = translate_args(args);
                let _res = script
                    .run(module, field, args)
                    .map_err(unexpected_failure)?;
                Ok(())
            }
            _ => {
                let message = format!("invoke {:?}", action);
                Err(Error::UnsupportedCommand(message))?
            }
        },

        // TODO: verify the exhaustion message is what we expected
        CommandKind::AssertExhaustion {
            ref action,
            message: _,
        } => match action {
            Action::Invoke {
                ref module,
                ref field,
                ref args,
            } => {
                println!("assert_exhaustion {:?} {} {:?}", module, field, args);
                let args = translate_args(args);
                let res = script.run(module, field, args);
                match res {
                    Ok(_) => Err(Error::UnexpectedSuccess)?,

                    Err(ScriptError::RuntimeError(RuntimeError::RuntimeFault(details), s)) => {
                        match details.trapcode {
                            Some(TrapCode::StackOverflow) => Ok(()),
                            e => {
                                let message = format!(
                                    "AssertExhaustion expects stack overflow, got {}. {:?}",
                                    s, e
                                );
                                Err(Error::UnexpectedFailure(message))
                            }
                        }
                    }

                    Err(e) => Err(unexpected_failure(e))?,
                }
            }
            _ => {
                let message = format!("invoke {:?}", action);
                Err(Error::UnsupportedCommand(message))?
            }
        },

        CommandKind::AssertReturn {
            ref action,
            ref expected,
        } => match action {
            Action::Invoke {
                ref module,
                ref field,
                ref args,
            } => {
                println!(
                    "assert_return (invoke {:?} {} {:?}) {:?}",
                    module, field, args, expected
                );
                let args = translate_args(args);
                let res = script
                    .run(module, field, args)
                    .map_err(unexpected_failure)?;
                check_retval(expected, res)?;
                Ok(())
            }
            _ => {
                let message = format!("non-invoke action");
                Err(Error::UnsupportedCommand(message))?
            }
        },
        CommandKind::AssertReturnCanonicalNan { action }
        | CommandKind::AssertReturnArithmeticNan { action } => match action {
            Action::Invoke {
                ref module,
                ref field,
                ref args,
            } => {
                println!("assert_nan");
                let args = translate_args(args);
                let res = script
                    .run(module, field, args)
                    .map_err(unexpected_failure)?;
                if res.as_f32().is_nan() || res.as_f64().is_nan() {
                    Ok(())
                } else {
                    let message = format!("expected NaN, got {}", res);
                    Err(Error::IncorrectResult(message))?
                }
            }
            _ => {
                let message = format!("non-invoke action");
                Err(Error::UnsupportedCommand(message))?
            }
        },
        CommandKind::AssertTrap { ref action, .. } => match action {
            Action::Invoke {
                module,
                field,
                args,
            } => {
                println!("assert_trap (invoke {:?} {} {:?})", module, field, args);
                let args = translate_args(args);
                let res = script.run(module, field, args);
                match res {
                    Err(ScriptError::RuntimeError(_luceterror, _)) => Ok(()),
                    Err(e) => Err(unexpected_failure(e)),
                    Ok(_) => Err(Error::UnexpectedSuccess)?,
                }
            }
            _ => {
                let message = format!("invoke {:?}", action);
                Err(Error::UnsupportedCommand(message))?
            }
        },
    }
}

fn check_retval(expected: &[Value], got: UntypedRetVal) -> Result<(), Error> {
    match expected.len() {
        0 => {}
        1 => match expected[0] {
            Value::I32(expected) => {
                if expected != got.as_i32() {
                    let message = format!("expected {}, got {}", expected, got.as_i32());
                    Err(Error::IncorrectResult(message))?
                }
            }
            Value::I64(expected) => {
                if expected != got.as_i64() {
                    let message = format!("expected {}, got {}", expected, got.as_i64());
                    Err(Error::IncorrectResult(message))?
                }
            }
            Value::F32(expected) => {
                if expected != got.as_f32() && !expected.is_nan() && !got.as_f32().is_nan() {
                    let message = format!("expected {}, got {}", expected, got.as_f32());
                    Err(Error::IncorrectResult(message))?
                }
            }
            Value::F64(expected) => {
                if expected != got.as_f64() && !expected.is_nan() && !got.as_f64().is_nan() {
                    let message = format!("expected {}, got {}", expected, got.as_f64());
                    Err(Error::IncorrectResult(message))?
                }
            }
            Value::V128(_) => {
                let message = format!("got unsupported SIMD V128 value");
                Err(Error::UnsupportedCommand(message))?;
            }
        },
        n => {
            let message = format!("{} expected return values not supported", n);
            Err(Error::UnsupportedCommand(message))?
        }
    }
    Ok(())
}

fn translate_args(args: &[Value]) -> Vec<Val> {
    let mut out = Vec::new();
    for a in args {
        let v = match a {
            Value::I32(ref i) => Val::U32(*i as u32),
            Value::I64(ref i) => Val::U64(*i as u64),
            Value::F32(ref f) => Val::F32(*f),
            Value::F64(ref f) => Val::F64(*f),
            Value::V128(_) => panic!("unsupported SIMD argument size: v128"),
        };
        out.push(v);
    }
    out
}
