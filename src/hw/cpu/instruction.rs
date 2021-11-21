use bitfield::bitfield;

bitfield! {
    pub struct Instruction(u32);
    impl Debug;

    #[inline(always)]
    pub special_opcode, _: 5, 0;
    #[inline(always)]
    pub opcode, _: 31, 26;
    #[inline(always)]
    pub rs, _: 25, 21;
    #[inline(always)]
    pub rt, _: 20, 16;
    #[inline(always)]
    pub rd, _: 15, 11;
    #[inline(always)]
    pub imm16, _: 15, 0;
    #[inline(always)]
    pub i16, simm16, _: 15, 0;
    #[inline(always)]
    pub imm5, _: 10, 6;
    #[inline(always)]
    pub imm26, _: 25, 0;
}