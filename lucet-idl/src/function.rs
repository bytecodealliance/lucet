use crate::error::ValidationError;
use crate::module::Module;
use crate::parser::{BindingRefSyntax, BindingSyntax, FuncArgSyntax};
use crate::types::{
    AbiType, BindDirection, BindingRef, DataTypeRef, FuncArg, FuncBinding, Location,
};
use std::collections::HashMap;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Position {
    Arg(usize),
    Ret(usize),
}

struct FuncValidator<'a> {
    // arg name to declaration location and argument position
    arg_names: HashMap<String, (Location, Position)>,
    // Arg positions index into this vector:
    args: Vec<FuncArg>,
    // Ret positions index into this vector:
    rets: Vec<FuncArg>,
    // binding name to
    binding_names: HashMap<String, (Location, usize)>,
    // arg name to binding location
    arg_use_sites: HashMap<String, Location>,
    bindings: Vec<FuncBinding>,
    location: &'a Location,
    module: &'a Module,
}

impl<'a> FuncValidator<'a> {
    fn new(location: &'a Location, module: &'a Module) -> Self {
        Self {
            arg_names: HashMap::new(),
            args: Vec::new(),
            rets: Vec::new(),
            binding_names: HashMap::new(),
            bindings: Vec::new(),
            arg_use_sites: HashMap::new(),
            location,
            module,
        }
    }
    fn introduce_arg_name(
        &mut self,
        arg_syntax: &FuncArgSyntax,
        position: Position,
    ) -> Result<FuncArg, ValidationError> {
        if let Some((previous_location, _)) = self.arg_names.get(&arg_syntax.name) {
            Err(ValidationError::NameAlreadyExists {
                name: arg_syntax.name.clone(),
                at_location: arg_syntax.location,
                previous_location: previous_location.clone(),
            })?;
        } else {
            self.arg_names.insert(
                arg_syntax.name.clone(),
                (arg_syntax.location.clone(), position),
            );
        }
        Ok(FuncArg {
            name: arg_syntax.name.clone(),
            type_: arg_syntax.type_.clone(),
        })
    }

    fn introduce_args(&mut self, args: &[FuncArgSyntax]) -> Result<(), ValidationError> {
        for (ix, a) in args.iter().enumerate() {
            let a = self.introduce_arg_name(a, Position::Arg(ix))?;
            self.args.push(a);
        }
        Ok(())
    }
    fn introduce_rets(&mut self, rets: &[FuncArgSyntax]) -> Result<(), ValidationError> {
        if rets.len() > 1 {
            Err(ValidationError::Syntax {
                expected: "at most one return value",
                location: self.location.clone(),
            })?
        }
        for (ix, r) in rets.iter().enumerate() {
            let r = self.introduce_arg_name(r, Position::Ret(ix))?;
            self.rets.push(r);
        }
        Ok(())
    }

    fn introduce_bindings(&mut self, bindings: &[BindingSyntax]) -> Result<(), ValidationError> {
        for (ix, binding) in bindings.iter().enumerate() {
            let b = self.introduce_binding(binding, ix)?;
            self.bindings.push(b);
        }
        Ok(())
    }

    fn introduce_binding(
        &mut self,
        binding: &BindingSyntax,
        position: usize,
    ) -> Result<FuncBinding, ValidationError> {
        // 1. make sure binding name is unique
        if let Some((previous_location, _)) = self.binding_names.get(&binding.name) {
            Err(ValidationError::NameAlreadyExists {
                name: binding.name.clone(),
                at_location: binding.location,
                previous_location: previous_location.clone(),
            })?;
        } else {
            self.binding_names
                .insert(binding.name.clone(), (binding.location.clone(), position));
        }

        // 2. resolve type_ SyntaxRef to a DataTypeRef
        let type_ = self.module.get_typeref(&binding.type_)?;

        // 3. typecheck the binding:
        let from = self.validate_binding_ref(&binding, &type_)?;

        Ok(FuncBinding {
            name: binding.name.clone(),
            direction: binding.direction.clone(),
            type_,
            from,
        })
    }

    fn get_arg(&self, arg_name: &String) -> Option<(Position, FuncArg)> {
        let (_, position) = self.arg_names.get(arg_name)?;
        match position {
            Position::Arg(ix) => Some((
                position.clone(),
                self.args.get(*ix).expect("in-bounds arg index").clone(),
            )),
            Position::Ret(ix) => Some((
                position.clone(),
                self.rets.get(*ix).expect("in-bounds ret index").clone(),
            )),
        }
    }

    fn validate_binding_arg_mapping(
        &mut self,
        name: &String,
        location: &Location,
    ) -> Result<(Position, FuncArg), ValidationError> {
        // Check that it refers to a valid arg:
        let pos_and_arg = self.get_arg(name).ok_or_else(|| ValidationError::Syntax {
            expected: "name of an argument or return value",
            location: location.clone(),
        })?;
        // Check that the arg has only been used once:
        if let Some(use_location) = self.arg_use_sites.get(name) {
            Err(ValidationError::BindingNameAlreadyBound {
                name: name.clone(),
                at_location: location.clone(),
                bound_location: use_location.clone(),
            })?;
        } else {
            self.arg_use_sites.insert(name.clone(), location.clone());
        }
        Ok(pos_and_arg)
    }

    fn validate_binding_ref(
        &mut self,
        binding: &BindingSyntax,
        target_type: &DataTypeRef,
    ) -> Result<BindingRef, ValidationError> {
        match &binding.from {
            // A pointer to a name is accepted:
            BindingRefSyntax::Ptr(bref) => match bref.deref() {
                BindingRefSyntax::Name(ref name) => {
                    let (position, funcarg) =
                        self.validate_binding_arg_mapping(name, &binding.location)?;
                    if funcarg.type_ != AbiType::I32 {
                        Err(ValidationError::BindingTypeError {
                            expected: "pointer bindings to be represented as an i32",
                            location: binding.location.clone(),
                        })?;
                    }
                    match position {
                        Position::Arg(_) => {
                            // all good! Arg pointers are valid for in, inout, or out binding.
                        }
                        Position::Ret(_) => {
                            if binding.direction != BindDirection::Out {
                                Err(ValidationError::BindingTypeError {
                                    expected: "return pointer must be output-only binding",
                                    location: binding.location.clone(),
                                })?;
                            }
                        }
                    }
                    Ok(BindingRef::Ptr(name.clone()))
                }
                _ => Err(ValidationError::Syntax {
                    expected: "pointer binding must be of form *arg",
                    location: binding.location.clone(),
                }),
            },
            // A slice of two names is accepted:
            BindingRefSyntax::Slice(ref ptr_ref, ref len_ref) => {
                match (ptr_ref.deref(), len_ref.deref()) {
                    (
                        BindingRefSyntax::Name(ref ptr_name),
                        BindingRefSyntax::Name(ref len_name),
                    ) => {
                        let (ptr_position, ptr_arg) =
                            self.validate_binding_arg_mapping(ptr_name, &binding.location)?;
                        if ptr_arg.type_ != AbiType::I32 {
                            Err(ValidationError::BindingTypeError {
                                expected: "slice pointer must be i32",
                                location: binding.location.clone(),
                            })?;
                        }
                        let (len_position, len_arg) =
                            self.validate_binding_arg_mapping(len_name, &binding.location)?;
                        if len_arg.type_ != AbiType::I32 {
                            Err(ValidationError::BindingTypeError {
                                expected: "slice len must be i32",
                                location: binding.location.clone(),
                            })?;
                        }
                        match (ptr_position, len_position) {
                            (Position::Arg(_), Position::Arg(_)) => {}
                            _ => {
                                Err(ValidationError::BindingTypeError {
                                    expected: "slice bindings must be inputs",
                                    location: binding.location.clone(),
                                })?;
                            }
                        }
                        Ok(BindingRef::Slice(ptr_name.to_owned(), len_name.to_owned()))
                    }
                    (
                        BindingRefSyntax::Name(ref _ptr_name),
                        BindingRefSyntax::Ptr(ref len_ptr_ref),
                    ) => match len_ptr_ref.deref() {
                        BindingRefSyntax::Name(_len_ptr_name) => {
                            unimplemented!("slice syntax [ptr, *len] for an output slice");
                        }
                        _ => Err(ValidationError::Syntax {
                            expected: "slice binding must be of form [ptr, len] or [ptr, *len]",
                            location: binding.location.clone(),
                        }),
                    },
                    _ => Err(ValidationError::Syntax {
                        expected: "slice binding must be of form [ptr, len] or [ptr, *len]",
                        location: binding.location.clone(),
                    }),
                }
            }
            // A bare name is accepted:
            BindingRefSyntax::Name(ref name) => {
                let (position, funcarg) =
                    self.validate_binding_arg_mapping(name, &binding.location)?;

                // make sure funcarg.type_ is a valid representation of target type
                match self.module.get_abi_repr(target_type) {
                    Some(target_repr) => {
                        if target_repr != funcarg.type_ {
                            Err(ValidationError::BindingTypeError {
                                expected: "binding type representation to match argument type",
                                location: binding.location.clone(),
                            })?;
                        }
                    }
                    None => {
                        Err(ValidationError::BindingTypeError {
                            expected: "binding type to be representable as value (try passing by reference instead)",
                            location: binding.location.clone(),
                        })?;
                    }
                }
                // Arg values must be in-only bindings, Ret values must be out-only bindings
                match position {
                    Position::Arg(_) => {
                        if binding.direction != BindDirection::In {
                            Err(ValidationError::BindingTypeError {
                                expected: "argument value must be input-only binding",
                                location: binding.location.clone(),
                            })?;
                        }
                    }
                    Position::Ret(_) => {
                        if binding.direction != BindDirection::Out {
                            Err(ValidationError::BindingTypeError {
                                expected: "return value must be output-only binding",
                                location: binding.location.clone(),
                            })?;
                        }
                    }
                }
                Ok(BindingRef::Value(name.clone()))
            }
        }
    }
}

pub fn validate_func_args(
    args: &[FuncArgSyntax],
    rets: &[FuncArgSyntax],
    bindings: &[BindingSyntax],
    location: &Location,
    module: &Module,
) -> Result<(Vec<FuncArg>, Vec<FuncArg>, Vec<FuncBinding>), ValidationError> {
    let mut validator = FuncValidator::new(location, module);
    validator.introduce_args(args)?;
    validator.introduce_rets(rets)?;
    validator.introduce_bindings(bindings)?;

    Ok((validator.args, validator.rets, validator.bindings))
}
