use super::render_tuple;
use crate::error::IDLError;
use crate::pretty_writer::PrettyWriter;
use crate::{Function, ParamPosition};
use heck::SnakeCase;

pub struct AbiCallBuilder<'a> {
    func: Function<'a>,
    before: Vec<String>,
    after: Vec<String>,
    args: Vec<Option<String>>,
    rets: Vec<Option<String>>,
}

impl<'a> AbiCallBuilder<'a> {
    pub fn new(func: Function<'a>) -> Self {
        let arg_len = func.args().collect::<Vec<_>>().len();
        let ret_len = func.rets().collect::<Vec<_>>().len();
        AbiCallBuilder {
            func,
            before: Vec::new(),
            after: Vec::new(),
            args: vec![None; arg_len],
            rets: vec![None; ret_len],
        }
    }

    pub fn param(&mut self, position: &ParamPosition, value: String) {
        match position {
            ParamPosition::Arg(n) => {
                self.args[*n] = Some(value);
            }
            ParamPosition::Ret(n) => {
                self.rets[*n] = Some(value);
            }
        }
    }

    pub fn before(&mut self, stmt: String) {
        self.before.push(stmt);
    }

    pub fn after(&mut self, stmt: String) {
        self.after.push(stmt);
    }

    pub fn render(&self, w: &mut PrettyWriter) -> Result<(), IDLError> {
        let name = self.func.name().to_snake_case();

        let arg_syntax = self
            .args
            .iter()
            .map(|v| {
                v.clone()
                    .ok_or(IDLError::InternalError("unconstructed abi arg"))
            })
            .collect::<Result<Vec<String>, IDLError>>()?
            .join(", ");
        let rets = self
            .rets
            .iter()
            .map(|v| {
                v.clone()
                    .ok_or(IDLError::InternalError("unconstructed abi ret"))
            })
            .collect::<Result<Vec<String>, IDLError>>()?;
        let ret_syntax = if rets.is_empty() {
            String::new()
        } else {
            assert_eq!(rets.len(), 1);
            format!("let {} = ", rets[0])
        };

        w.writelns(&self.before);

        w.writeln(format!(
            "{}unsafe {{ abi::{}({}) }};",
            ret_syntax, name, arg_syntax
        ));

        w.writelns(&self.after);

        Ok(())
    }
}

pub struct FuncBuilder {
    name: String,
    error_type: String,
    args: Vec<String>,
    ok_types: Vec<String>,
    ok_values: Vec<String>,
}

impl FuncBuilder {
    pub fn new(name: String, error_type: String) -> Self {
        FuncBuilder {
            name,
            error_type,
            args: Vec::new(),
            ok_types: Vec::new(),
            ok_values: Vec::new(),
        }
    }

    pub fn arg(&mut self, arg: String) {
        self.args.push(arg)
    }

    pub fn ok_type(&mut self, arg: String) {
        self.ok_types.push(arg);
    }

    pub fn ok_value(&mut self, val: String) {
        self.ok_values.push(val);
    }

    pub fn render<F>(&self, w: &mut PrettyWriter, body: F) -> Result<(), IDLError>
    where
        F: FnOnce(&mut PrettyWriter) -> Result<(), IDLError>,
    {
        if self.ok_types.len() != self.ok_values.len() {
            Err(IDLError::InternalError(
                "func builder ok types do not match ok values",
            ))?;
        }
        let arg_syntax = self.args.join(", ");
        let ret_syntax = format!(
            "Result<{},{}>",
            render_tuple(&self.ok_types),
            self.error_type
        );
        w.writeln(format!(
            "pub fn {}({}) -> {} {{",
            self.name, arg_syntax, ret_syntax
        ))
        .indent();
        body(w)?;
        w.writeln(format!("Ok({})", render_tuple(&self.ok_values)));
        w.eob().writeln("}".to_owned());
        Ok(())
    }
}
