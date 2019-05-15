use super::*;
use std::cmp;

pub fn generate(
    gen: &mut CGenerator,
    module: &Module,
    dt: &Named<DataType>,
    struct_: &StructDataType,
) -> Result<(), IDLError> {
    let dtname = &dt.name.name;
    gen.w
        .write_line(format!("struct {} {{", dtname).as_bytes())?;
    let mut w_block = gen.w.new_block();
    let mut offset: usize = 0;
    for member in struct_.members.iter() {
        w_block.indent()?;
        w_block.write(gen.type_name(module, &member.type_).as_bytes())?;
        w_block.space()?;
        w_block.write(member.name.as_bytes())?;
        w_block.write(b";")?;
        w_block.eol()?;

        offset += member.repr_size;
    }
    gen.w.write_line(b"};")?;
    gen.w.eob()?;

    let struct_size = offset;

    // Add assertions to check that the target platform matches the expected alignment
    // Also add a macro definition for the structure size
    // Skip the first member, as it will always be at the beginning of the structure
    for (i, member) in struct_.members.iter().enumerate().skip(1) {
        gen.w.write_line(
            format!(
                "_Static_assert(offsetof(struct {}, {}) == {}, \"unexpected offset\");",
                dtname, member.name, member.offset
            )
            .as_bytes(),
        )?;
    }
    gen.w.write_line(
        format!(
            "_Static_assert(sizeof(struct {}) == {}, \"unexpected structure size\");",
            dtname, struct_size
        )
        .as_bytes(),
    )?;
    gen.w.eob()?;
    macros::define(gen, "BYTES", dtname, struct_size)?;
    gen.w.eob()?;

    Ok(())
}
