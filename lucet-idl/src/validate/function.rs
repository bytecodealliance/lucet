use super::names::ModNamesBuilder;
use crate::parser::{BindingDirSyntax, BindingRefSyntax, BindingSyntax, FuncArgSyntax};
use crate::repr::{
    ArgIx, BindingDirection, BindingFromRepr, BindingIx, BindingRepr, FuncIx, FuncRepr,
    ModuleFuncsRepr, ParamIx, ParamRepr, RetIx,
};
use crate::{AbiType, AtomType, Datatype, Location, Module, ValidationError};
use cranelift_entity::{EntityRef, PrimaryMap};
use std::collections::HashMap;
use std::ops::Deref;

pub struct FunctionModuleBuilder<'a> {
    env: Module<'a>,
    names: &'a ModNamesBuilder,
    funcs: PrimaryMap<FuncIx, FuncRepr>,
}

impl<'a> FunctionModuleBuilder<'a> {
    pub fn new(env: Module<'a>, names: &'a ModNamesBuilder) -> Self {
        Self {
            env,
            names,
            funcs: PrimaryMap::new(),
        }
    }

    pub fn introduce_func(
        &mut self,
        name: &str,
        args: &[FuncArgSyntax],
        rets: &[FuncArgSyntax],
        bindings: &[BindingSyntax],
        location: &Location,
    ) -> Result<(), ValidationError> {
        let mut validator = FuncValidator::new(location, &self.env);
        validator.introduce_args(args)?;
        validator.introduce_rets(rets)?;
        validator.introduce_bindings(bindings)?;

        let defined_ix = self.funcs.push(FuncRepr {
            args: validator.args,
            rets: validator.rets,
            bindings: validator.bindings,
        });
        let declared_ix = self.names.func_from_name(name).expect("declared func");
        assert_eq!(
            defined_ix, declared_ix,
            "funcs defined in different order than declared"
        );
        Ok(())
    }

    pub fn build(self) -> ModuleFuncsRepr {
        assert_eq!(
            self.names.funcs.len(),
            self.funcs.len(),
            "each func declared has been defined"
        );
        ModuleFuncsRepr {
            names: self.names.funcs.clone(),
            funcs: self.funcs,
        }
    }
}

struct FuncValidator<'a> {
    // arg name to declaration location and argument position
    param_names: HashMap<String, (Location, ParamIx)>,
    // Arg positions index into this vector:
    args: PrimaryMap<ArgIx, ParamRepr>,
    // Ret positions index into this vector:
    rets: PrimaryMap<RetIx, ParamRepr>,
    // binding name to
    binding_names: HashMap<String, (Location, BindingIx)>,
    // param position to binding syntax
    bindings: PrimaryMap<BindingIx, BindingRepr>,
    param_binding_sites: HashMap<ParamIx, Location>,
    location: &'a Location,
    module: &'a Module<'a>,
}

impl<'a> FuncValidator<'a> {
    fn new(location: &'a Location, module: &'a Module<'a>) -> Self {
        Self {
            param_names: HashMap::new(),
            args: PrimaryMap::new(),
            rets: PrimaryMap::new(),
            binding_names: HashMap::new(),
            bindings: PrimaryMap::new(),
            param_binding_sites: HashMap::new(),
            location,
            module,
        }
    }
    fn introduce_param_name(
        &mut self,
        arg_syntax: &FuncArgSyntax,
        position: ParamIx,
    ) -> Result<ParamRepr, ValidationError> {
        if let Some((previous_location, _)) = self.param_names.get(&arg_syntax.name) {
            Err(ValidationError::NameAlreadyExists {
                name: arg_syntax.name.clone(),
                at_location: arg_syntax.location,
                previous_location: previous_location.clone(),
            })?;
        } else {
            self.param_names.insert(
                arg_syntax.name.clone(),
                (arg_syntax.location.clone(), position),
            );
        }
        Ok(ParamRepr {
            name: arg_syntax.name.clone(),
            type_: arg_syntax.type_.clone(),
        })
    }

    fn introduce_args(&mut self, args: &[FuncArgSyntax]) -> Result<(), ValidationError> {
        for (ix, arg) in args.iter().enumerate() {
            let arg_ix = ArgIx::new(ix);
            let a = self.introduce_param_name(arg, ParamIx::Arg(arg_ix))?;
            let pushed_arg_ix = self.args.push(a);
            assert_eq!(arg_ix, pushed_arg_ix);
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
            let ret_ix = RetIx::new(ix);
            let r = self.introduce_param_name(r, ParamIx::Ret(ret_ix))?;
            let pushed_ret_ix = self.rets.push(r);
            assert_eq!(ret_ix, pushed_ret_ix);
        }
        Ok(())
    }

    fn introduce_bindings(&mut self, bindings: &[BindingSyntax]) -> Result<(), ValidationError> {
        for (ix, binding) in bindings.iter().enumerate() {
            let ix = BindingIx::new(ix);
            let b = self.introduce_binding(binding, ix)?;
            let pushed_ix = self.bindings.push(b);
            assert_eq!(ix, pushed_ix);
        }
        for (ix, arg) in self.args.iter() {
            let position = ParamIx::Arg(ix);
            if !self.param_binding_sites.contains_key(&position) {
                self.bindings
                    .push(self.implicit_value_binding(&arg, position)?);
            }
        }
        for (ix, ret) in self.rets.iter() {
            let position = ParamIx::Ret(ix);
            if !self.param_binding_sites.contains_key(&position) {
                self.bindings
                    .push(self.implicit_value_binding(&ret, position)?);
            }
        }
        Ok(())
    }

    fn introduce_binding(
        &mut self,
        binding: &BindingSyntax,
        ix: BindingIx,
    ) -> Result<BindingRepr, ValidationError> {
        // 1. make sure binding name is unique
        if let Some((previous_location, _)) = self.binding_names.get(&binding.name) {
            Err(ValidationError::NameAlreadyExists {
                name: binding.name.clone(),
                at_location: binding.location,
                previous_location: previous_location.clone(),
            })?;
        } else {
            self.binding_names
                .insert(binding.name.clone(), (binding.location.clone(), ix));
        }

        // 2. resolve type_ SyntaxRef to a Datatype
        let type_ = self
            .module
            .datatype_by_syntax(&binding.type_)
            .ok_or_else(|| ValidationError::NameNotFound {
                name: format!("{:?}", binding.type_), // XXX FIXME
                use_location: binding.location,
            })?;

        // 3. typecheck the binding:
        let from = self.validate_binding_ref(&binding, &type_)?;

        // 4. direction from syntax:
        let direction = match binding.direction {
            BindingDirSyntax::In => BindingDirection::In,
            BindingDirSyntax::InOut => BindingDirection::InOut,
            BindingDirSyntax::Out => BindingDirection::Out,
        };

        Ok(BindingRepr {
            name: binding.name.clone(),
            type_: type_.id(),
            direction,
            from,
        })
    }

    fn implicit_value_binding(
        &self,
        arg: &ParamRepr,
        position: ParamIx,
    ) -> Result<BindingRepr, ValidationError> {
        // 1. make sure binding name is unique. We're re-using the arg name
        // for the binding. If another binding overlapped with the arg name,
        // it is now at fault. (complicated, huh... :/)
        if let Some((previous_location, _)) = self.binding_names.get(&arg.name) {
            let (arg_location, _) = self.param_names.get(&arg.name).expect("arg introduced");
            Err(ValidationError::BindingNameAlreadyBound {
                name: arg.name.clone(),
                at_location: previous_location.clone(),
                bound_location: arg_location.clone(),
            })?;
        }

        // 2. resolve type
        let type_ = AtomType::from(arg.type_).datatype_id();

        // 3. no need to validate ref- we can construct it ourselves
        let from = BindingFromRepr::Value(position);

        // 4. direction depends on whether param is an arg or ret
        let direction = match position {
            ParamIx::Arg(_) => BindingDirection::In,
            ParamIx::Ret(_) => BindingDirection::Out,
        };

        Ok(BindingRepr {
            name: arg.name.clone(),
            type_,
            direction,
            from,
        })
    }

    fn get_arg(&self, arg_name: &String) -> Option<(ParamIx, ParamRepr)> {
        let (_, position) = self.param_names.get(arg_name)?;
        match position {
            ParamIx::Arg(ix) => Some((
                *position,
                self.args.get(*ix).expect("in-bounds arg index").clone(),
            )),
            ParamIx::Ret(ix) => Some((
                *position,
                self.rets.get(*ix).expect("in-bounds ret index").clone(),
            )),
        }
    }

    fn validate_binding_arg_mapping(
        &mut self,
        name: &String,
        location: &Location,
    ) -> Result<(ParamIx, ParamRepr), ValidationError> {
        // Check that it refers to a valid arg:
        let (position, arg) = self.get_arg(name).ok_or_else(|| ValidationError::Syntax {
            expected: "name of an argument or return value",
            location: location.clone(),
        })?;
        // Check that the arg has only been used once:
        if let Some(use_location) = self.param_binding_sites.get(&position) {
            Err(ValidationError::BindingNameAlreadyBound {
                name: name.clone(),
                at_location: location.clone(),
                bound_location: use_location.clone(),
            })?;
        } else {
            self.param_binding_sites
                .insert(position.clone(), location.clone());
        }
        Ok((position, arg))
    }

    fn validate_binding_ref(
        &mut self,
        binding: &BindingSyntax,
        target_type: &Datatype<'a>,
    ) -> Result<BindingFromRepr, ValidationError> {
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
                        ParamIx::Arg(_) => {
                            // all good! Arg pointers are valid for in, inout, or out binding.
                        }
                        ParamIx::Ret(_) => {
                            Err(ValidationError::BindingTypeError {
                                expected: "return value cannot be bound to pointer",
                                location: binding.location.clone(),
                            })?;
                        }
                    }
                    Ok(BindingFromRepr::Ptr(position))
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
                        match (&ptr_position, &len_position) {
                            (ParamIx::Arg(_), ParamIx::Arg(_)) => {}
                            _ => {
                                Err(ValidationError::BindingTypeError {
                                    expected: "slice bindings must be inputs",
                                    location: binding.location.clone(),
                                })?;
                            }
                        }
                        Ok(BindingFromRepr::Slice(ptr_position, len_position))
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
                match target_type.abi_type() {
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
                    ParamIx::Arg(_) => {
                        if binding.direction != BindingDirSyntax::In {
                            Err(ValidationError::BindingTypeError {
                                expected: "argument value must be input-only binding",
                                location: binding.location.clone(),
                            })?;
                        }
                    }
                    ParamIx::Ret(_) => {
                        if binding.direction != BindingDirSyntax::Out {
                            Err(ValidationError::BindingTypeError {
                                expected: "return value must be output-only binding",
                                location: binding.location.clone(),
                            })?;
                        }
                    }
                }
                Ok(BindingFromRepr::Value(position))
            }
        }
    }
}