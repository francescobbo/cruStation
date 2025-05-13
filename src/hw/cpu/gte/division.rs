//! This file has been pretty much copied from Mednafen.
//!
//! The GTE hardware computes H / SZ3 using this Unsigned Newton-Raphson
//! division algorithm.
//!
//! The UNR_TABLE is actually hardcoded in the GTE hardware, but we compute it
//! here for reference.
//!
//! TODO: fully understand the algorithm, its magic numbers and properly
//! document it.

/// Approximate division of two 16-bit unsigned integers using the Unsigned
/// Newton-Raphson division algorithm.
///
/// Returns a u32 in 1.15.16 fixed point format.
///
/// This tricky function has a few interesting limitations:
/// - It works only if the dividend is less than twice the divisor.
///   - 2 / 100 is ok (~0.02), 100 / 2 overflows (~1.9999)
/// - If any operand has a sign bit set the result is pretty much garbage.
///
/// # Returns
///
/// Returns a tuple with the 1.15.16 fixed point result and a boolean indicating
/// if the result overflowed.
///
/// # Example
///
/// ```
/// # use psx::gte::division::division;
/// let (result, _) = division(1, 10);
/// // 0x0.199a (fixed point) is ~0.100006104
/// assert_eq!(result, 0x0_199a);
/// ```
///
/// ```
/// # use psx::gte::division::division;
/// let (result, of) = division(30, 10); // This overflows as 30 > 10 * 2
/// // 0x1.ffff (fixed point) is ~1.999984741
/// assert_eq!(result, 0x1_ffff);
/// assert_eq!(of, true);
/// ```
pub fn division(dividend: u16, divisor: u16) -> (u32, bool) {
    if (dividend as u64) >= (divisor as u64 * 2) {
        return (0x1_ffff, true);
    }

    let shift = divisor.leading_zeros();
    let dividend = (dividend as u64) << shift;
    let divisor = divisor << shift;

    let reciprocal = reciprocal(divisor);
    let result = ((dividend * reciprocal) + 0x8000) >> 16;

    if result > 0x1_ffff {
        (0x1_ffff, false)
    } else {
        (result as u32, false)
    }
}

/// Approximate reciprocal of a 16-bit unsigned integer using the Unsigned
/// Newton-Raphson division algorithm.
fn reciprocal(divisor: u16) -> u64 {
    let index = (((divisor & 0x7fff) + 0x40) >> 7) as usize;
    let factor = (UNR_TABLE[index] as u64 + 0x101) as i64;
    let tmp = (((divisor as i64) * -factor) + 0x80) >> 8;

    (((factor * (0x2_0000 + tmp)) + 0x80) >> 8) as u64
}

/// Unsigned Newton-Raphson reciprocal table.
pub const UNR_TABLE: [u8; 0x101] = {
    let mut table = [0; 0x101];

    let mut i = 0;
    while i < 0x101 {
        let val = (0x4_0000 / (i as i64 + 0x100) + 1) / 2 - 0x101;
        table[i] = if val > 0 { val } else { 0 } as u8;

        i += 1;
    }

    table
};

#[cfg(test)]
mod test {
    use super::division;

    fn fixed_point_to_float(value: u64) -> f32 {
        let integral_part = (value >> 16) as f32;
        let fractional_part = (value & 0xffff) as f32 / 0x1_0000 as f32;

        integral_part + fractional_part
    }

    #[test]
    fn test_division() {
        let (result, _) = division(1, 10);

        println!("{:x}", result);

        let result = fixed_point_to_float(result as u64);

        // verify that the result is within an acceptable error margin
        assert!((result - 1.0 / 10.0).abs() < 0.001);
    }

    #[test]
    fn test_division_overflow() {
        let (result, of) = division(30, 10);

        let result = fixed_point_to_float(result as u64);

        // the result is the overflow value
        assert!((result - 1.999).abs() < 0.001);

        // overflow flag is set
        assert!(of);
    }
}
