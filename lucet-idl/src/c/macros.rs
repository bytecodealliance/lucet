use super::*;

// Return the name of a macro according to a name and a prefix

pub fn macro_for(prefix: &str, name: &str) -> String {
    let mut macro_name = String::new();
    macro_name.push_str(&prefix.to_uppercase());
    macro_name.push('_');
    let mut previous_was_uppercase = name.chars().nth(0).expect("Empty name").is_uppercase();
    for c in name.chars() {
        let is_uppercase = c.is_uppercase();
        if is_uppercase != previous_was_uppercase {
            macro_name.push('_');
        }
        for uc in c.to_uppercase() {
            macro_name.push(uc);
        }
        previous_was_uppercase = is_uppercase;
    }
    macro_name
}

// Generate a macro definition

pub fn define<W: Write, V: ToString>(
    _cgenerator: &mut CGenerator,
    pretty_writer: &mut PrettyWriter<W>,
    prefix: &str,
    name: &str,
    value: V,
) -> Result<(), IDLError> {
    let macro_name = macro_for(prefix, name);
    let mut pretty_writer_preprocessor = pretty_writer.new_from_writer();
    pretty_writer_preprocessor
        .write_line(format!("#define {} {}", macro_name, value.to_string()).as_ref())?;
    Ok(())
}

// Return a macro name for a type reference

pub fn macro_for_data_type_ref(
    module: &Module,
    prefix: &str,
    data_type_ref: &DataTypeRef,
) -> String {
    match data_type_ref {
        DataTypeRef::Atom(atom_type) => {
            let native_type_size = CAtom::from(*atom_type).native_type_size;
            format!("{}", native_type_size)
        }
        DataTypeRef::Defined(data_type_id) => {
            let data_type_entry = module.get_datatype(*data_type_id).expect("defined datatype");
            macro_for(prefix, &data_type_entry.name.name)
        }
    }
}
