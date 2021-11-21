/// This file has been pretty much copied from Mednafen.
/// One day I hope to understand what's going on here better
/// and to properly document this.

pub const UNR_TABLE: [u8; 0x101] = {
    let mut table: [u8; 0x101] = [0; 0x101];

    let mut i = 0;
    while i < 0x101 {
        let val = (0x40000 / (i as i64 + 0x100) + 1) / 2 - 0x101;
        table[i] = if val > 0 { val } else { 0 } as u8;

        i += 1;
    }

    table
};

pub fn division(dividend: u16, divisor: u16) -> (u32, bool) {
    if (divisor as u64 * 2) <= (dividend as u64) {
        return (0x1ffff, true);
    }

    let shift = divisor.leading_zeros();
    let dividend = (dividend as u64) << shift;
    let divisor = divisor << shift;
    
    let reciprocal = reciprocal(divisor);
    let result = ((dividend * reciprocal) + 0x8000) >> 16;

    if result > 0x1ffff {
        (0x1ffff, false)
    } else {
        (result as u32, false)
    }
}

fn reciprocal(divisor: u16) -> u64 {
    let index = (((divisor & 0x7fff) + 0x40) >> 7) as usize;
    let factor = (UNR_TABLE[index] as u64 + 0x101) as i64;
    let tmp = (((divisor as i64) * -factor) + 0x80) >> 8;
    
    (((factor * (0x20000 + tmp)) + 0x80) >> 8) as u64
}
