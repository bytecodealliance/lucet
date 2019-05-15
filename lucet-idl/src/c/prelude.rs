use super::*;

pub fn generate(pretty_writer: &mut PrettyWriter, target: Target) -> Result<(), IDLError> {
    let prelude = r"
#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>";
    for line in prelude.lines() {
        pretty_writer.write_line(line.as_ref())?;
    }
    pretty_writer.eob()?;
    Ok(())
}
