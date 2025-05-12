#[derive(Debug, Default)]
#[repr(C)]
pub struct PsxExeHeader {
    signature: [u8; 8],
    zero1: [u8; 8],
    pub pc: u32,
    pub r28: u32,
    pub destination: u32,
    pub size: u32,
    zero2: [u32; 2],
    memfill_address: u32,
    memfill_size: u32,
    pub r29_base: u32,
    pub r29_offset: u32,
}