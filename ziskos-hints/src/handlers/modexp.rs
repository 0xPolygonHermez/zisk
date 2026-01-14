use crate::{handlers::validate_hint_length, zisklib};

/// Read a length-prefixed field from hint data
#[inline]
fn read_field<'a>(data: &'a [u64], pos: &mut usize) -> Result<&'a [u64], String> {
    let len = *data.get(*pos).ok_or("MODEXP hint data too short")? as usize;
    *pos += 1;
    let field = data.get(*pos..*pos + len).ok_or("MODEXP hint data too short")?;
    *pos += len;
    Ok(field)
}

// Processes a MODEXP hint.
#[inline]
pub fn modexp_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    let mut pos = 0;
    let base = read_field(data, &mut pos)?;
    let exp = read_field(data, &mut pos)?;
    let modulus = read_field(data, &mut pos)?;

    validate_hint_length(data, pos, "MODEXP")?;

    let mut processed_hints = Vec::new();
    zisklib::modexp_u64(base, exp, modulus, &mut processed_hints);

    Ok(processed_hints)
}
