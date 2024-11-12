use crustationlogger::*;

/// The MIPS R3000A System Coprocessor
///
/// The most important task of this chip is Exception handling.
/// It also implements some hardware debugging utilities.
///
/// # Access
/// Access is always allowed in Kernel mode (see cop0r12.b1).
/// Code running in user mode (is this even used on the PS?), can only access
/// COP0 if cop0r12.b28 is set. If that's not the case a Coprocessor Unusable
/// Exception is raised when accessing registers 0 to 15.
///
/// # Available registers
/// The ISA supports 64 registers, but the hardware will only allow use of the
/// first 16.
///
/// Writes to registers 16 to 31 are ignored, and reads return weird values.
/// Writes to registers 0, 1, 2, 4, 10 and 32 to 63 are ignored, and reads
/// yield a Coprocessor Unavailable Exception.
///
/// ## Used registers
/// \# | Name  | Description
/// ---|-------|-----------
///  3 | BPC   | Breakpoint on PC
///  5 | BDA   | Breakpoint on data access
///  6 | TAR   | Jump destination
///  7 | DCIC  | Breakpoint control
///  8 | BADA  | Bad address
///  9 | BDAM  | Data Access breakpoint mask
/// 11 | BPCM  | PC breakpoint mask
/// 12 | SR    | System status register
/// 13 | CAUSE | Last exception description
/// 14 | EPC   | Return address from exception
/// 15 | PRID  | Processor ID
///
/// ## Registers 16 to 31 read garbage
/// When reading one of the garbage registers shortly after reading a valid
/// cop0 register, the garbage value is usually the same as that of the valid
/// register. When doing the read later on, the return value is usually
/// 0x0000_0020, or when reading much later it returns 0x0000_0040, or even
/// 0x0000_0100
///
/// ## SR: System Status Register
/// Bits | Description
/// -----|------------
///    0 | Interrupts enabled?
///    1 | User mode?
/// 2, 4 | Two "interrupts enabled" backup slots for nested exceptions
/// 3, 5 | Two "user mode" backup slots for nested exceptions
/// 8-15 | Interrupt mask. Bits set are interrupt allowed to run
///   16 | Cache isolated?
///   22 | Boot exception vectors
///   28 | COP0 enabled?
///   30 | COP2 enabled?
///
/// b16 disconnects the main bus, only allowing reads and writes on the caches.
/// b22 determines wheter exceptions are handled in the ROM (0xbfcx_xxxx) or
/// RAM (0xa00x_xxxx).
/// b28 set to 0 prevents COP0 access in user mode.
/// b30 set to 0 prevents any GTE use.
///
/// ### Exception behaviour and RFE
/// On an exception bits 2 and 3 are moved to 4 and 5; bits 0 and 1 to 2 and 3.
/// Bits 0 and 1 are set to 0 (kernel mode, interrupts disabled)
///
/// RFE moves bits the other way around.
///
/// ## CAUSE: Exception cause register
///  Bits | Description
/// ------|------------
///   2-6 | Exception description number (Interrupt, Bus Error, System Call...)
///  8-15 | Bitfield indicating which interrupts are waiting to be processed
/// 28-29 | Coprocessor number that caused a CoprocessorUnusable exception
///    31 | Indicates whether the last exception happened in a branch delay slot
///
/// Note: bits 8 and 9 are freely R/W. The CPU will use bit10 to raise an
/// interrupt based on I_STAT and I_MASK. The rest seem unused.
///
/// ## EPC: Return address from exception
/// Contains the PC address to return to after handling an exception.
///
/// ## PRID: Processor ID
/// A fixed 2
///
/// # TODOs
/// - Breakpoints (r3, r5, r7, r8, r9, r11)
/// - Check read behaviour of r6 (TAR) and garbage (r16 - r31)
pub struct Cop0 {
    logger: Logger,

    pub regs: [u32; 16],

    /// Mirror of SR.b0
    pub interrupts_enabled: bool,

    /// Mirror of SR.b1
    pub is_user: bool,

    /// Mirror of SR.b16
    pub isolate_cache: bool,

    /// Mirror of SR.b22
    pub boot_vectors: bool,

    /// Mirror of SR.b28
    pub cop0_enabled: bool,

    /// Mirror of SR.b29
    pub cop1_enabled: bool,

    /// Mirror of SR.b30
    pub cop2_enabled: bool,

    /// Mirror of SR.b31
    pub cop3_enabled: bool,
}

/// List of supported COP0 exceptions.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Exception {
    Interrupt = 0,
    AddressErrorLoad = 4,
    AddressErrorStore = 5,
    Syscall = 8,
    Breakpoint = 9,
    ReservedInstruction = 10,
    CoprocessorUnusable = 11,
    Overflow = 12,
}

/// Masks of bits allowed to be set by the CPU with MTC
const WRITE_MASKS: [u32; 16] = [
    0,
    0,
    0,
    0xffff_ffff,
    0,
    0xffff_ffff,
    0,
    0xff80_f03f,
    0,
    0xffff_ffff,
    0,
    0xffff_ffff,
    0xf04f_ff3f,
    0x0000_0300,
    0,
    0,
];

const BDAM: usize = 9;
const BPCM: usize = 11;
const STATUS: usize = 12;
const CAUSE: usize = 13;
const EPC: usize = 14;
const PRID: usize = 15;

impl Cop0 {
    /// Creates a new Cop0 with registers filled as if just booted
    pub fn new() -> Cop0 {
        let mut regs = [0; 16];
        regs[BDAM] = 0xffff_ffff;
        regs[BPCM] = 0xffff_ffff;
        regs[STATUS] = 0x0040_0000;
        regs[PRID] = 0x0000_0002;

        Cop0 {
            logger: Logger::new("COP0", Level::Info),

            regs,
            interrupts_enabled: false,
            is_user: false,
            isolate_cache: false,
            boot_vectors: true,
            cop0_enabled: false,
            cop1_enabled: false,
            cop2_enabled: false,
            cop3_enabled: false,
        }
    }

    /// Reads a COP0 register.
    /// A `None` value returned means that the CPU should raise a CU error.
    pub fn read_reg(&self, index: u32) -> Option<u32> {
        if self.is_user && !self.cop0_enabled && index < 16 {
            return None;
        }

        match index {
            0..=2 | 4 | 10 | 32..=63 => {
                err!(self.logger, "Read from an unavailable register r{}", index);
                None
            }
            16..=31 => {
                warn!(self.logger, "Read from a garbage register r{}", index);

                // TODO: investigate if the weird behaviour of these registers
                // is actually used by anything.
                // And if so, try to understand better how it works.
                Some(0)
            }
            _ => Some(self.regs[index as usize]),
        }
    }

    /// Writes to a COP0 register.
    /// An `Err` value returned means that the CPU should raise a CU error.
    pub fn write_reg(&mut self, index: u32, value: u32) -> Result<(), ()> {
        if self.is_user && !self.cop0_enabled && index < 16 {
            err!(self.logger, "Write attempt in user mode");
            return Err(());
        }

        let index = index as usize;

        match index {
            0..=15 => {
                self.regs[index] &= !WRITE_MASKS[index];
                self.regs[index] |= value & WRITE_MASKS[index];

                debug!(self.logger, "Set r{} to {:08x}", index, self.regs[index]);

                if index == STATUS {
                    self.update_status();
                }

                Ok(())
            }
            16..=31 => {
                warn!(self.logger, "Write to a garbage register r{}", index);
                Ok(())
            }
            _ => {
                err!(self.logger, "Write to an unavailable register r{}", index);
                Err(())
            }
        }
    }

    /// Executes an operation on COP0. Specifically only `RFE` is implemented.
    /// An `Err` value returned means that the CPU should raise a CU error.
    pub fn execute(&mut self, operation: u32) -> Result<(), Exception> {
        if self.is_user && !self.cop0_enabled {
            err!(self.logger, "Operation attempt in user mode");
            return Err(Exception::CoprocessorUnusable);
        }

        match operation & 0x3f {
            0x01 | 0x02 | 0x06 | 0x08 => {
                // TLBR / TLBWI / TLBWR / TLBP
                // The PlayStation does not have a TLB.
                err!(self.logger, "TLB instructions are not available");
                Err(Exception::ReservedInstruction)
            }
            0x10 => {
                // RFE
                let mode = (self.regs[STATUS] & 0x3c) >> 2;
                self.regs[STATUS] &= !0xf;
                self.regs[STATUS] |= mode;

                self.update_status();

                debug!(self.logger, "RFE. SR: {:08x}", self.regs[STATUS]);

                Ok(())
            }
            _ => {
                err!(self.logger, "Invalid operation {:08x}", operation);

                // Apparently, this should not raise an exception
                Ok(())
            }
        }
    }

    /// Enters an exception status.
    ///
    /// The low 4 bits in the SR register are shifted up by two bits.
    /// The low 2 bits of the SR register are set to zero, kernel mode, with
    /// interrupts disabled.
    ///
    /// The CAUSE register is set with the exception cause, optionally with a
    /// Coprocessor number, if the exception was CoprocessorUnusable, and with
    /// b31 set if the exception happened in a branch delay slot.
    pub fn enter_exception(
        &mut self,
        cause: Exception,
        instruction_pc: u32,
        is_delay_slot: bool,
        cop_number: u32,
    ) {
        // Handle low 4 bits
        let mode = self.regs[STATUS] & 0xf;
        self.regs[STATUS] &= !0x3f;
        self.regs[STATUS] |= mode << 2;

        // Clear the fields we are going to change (or are fixed-zero)
        self.regs[CAUSE] &= 0xff00;

        // Store the cause
        self.regs[CAUSE] |= (cause as u32) << 2;

        // Remember which COP caused the fault
        if cause == Exception::CoprocessorUnusable {
            self.regs[CAUSE] |= cop_number << 28;
        }

        // Remember if it was a branch delay
        if is_delay_slot {
            self.regs[CAUSE] |= (1 << 31) as u32;
        }

        // Remeber the return address
        // If it was a branch delay slot, the branch must be re-executed too,
        // so go back by a word.
        self.regs[EPC] = if is_delay_slot {
            instruction_pc.wrapping_sub(4)
        } else {
            instruction_pc
        };

        self.update_status();

        debug!(
            self.logger,
            "Entering exception {:?}. SR: {:08x}, CAUSE: {:08x}, EPC: {:08x}",
            cause,
            self.regs[STATUS],
            self.regs[CAUSE],
            self.regs[EPC]
        );
    }

    /// Returns the PC that is expected to handle a given exception
    /// The returned value will depend on the kind of exception and on SR.b22
    pub fn exception_handler(&self, cause: Exception) -> u32 {
        if self.boot_vectors {
            if cause == Exception::Breakpoint {
                0xbfc0_0140
            } else {
                0xbfc0_0180
            }
        } else if cause == Exception::Breakpoint {
            0x8000_0040
        } else {
            0x8000_0080
        }
    }

    /// An interrupt has to be handled if:
    /// - The master interrupt flag is set (SR.b0)
    /// - One or more interrupts are requested in CAUSE.b8-15
    /// - The same interrupts are allowed in SR.b8-15
    pub fn should_interrupt(&self) -> bool {
        if self.interrupts_enabled {
            let pending = self.regs[CAUSE] & 0xff00;
            let mask = self.regs[STATUS] & 0xff00;

            pending & mask != 0
        } else {
            false
        }
    }

    /// Sets a bit in the CAUSE register to indicate an IRQ
    pub fn request_interrupt(&mut self, n: u32) {
        self.regs[CAUSE] |= 1 << n;
    }

    /// Clears a bit in the CAUSE register to acknowledge an IRQ
    pub fn clear_interrupt(&mut self, n: u32) {
        self.regs[CAUSE] &= !(1 << n);
    }

    /// Updates Cop0 struct flags based on the Status Register (rSTATUS).
    fn update_status(&mut self) {
        self.interrupts_enabled = self.regs[STATUS] & 1 != 0;
        self.is_user = self.regs[STATUS] & (1 << 1) != 0;
        self.isolate_cache = self.regs[STATUS] & (1 << 16) != 0;
        self.boot_vectors = self.regs[STATUS] & (1 << 22) != 0;
        self.cop0_enabled = self.regs[STATUS] & (1 << 28) != 0;
        self.cop1_enabled = self.regs[STATUS] & (1 << 29) != 0;
        self.cop2_enabled = self.regs[STATUS] & (1 << 30) != 0;
        self.cop3_enabled = self.regs[STATUS] & (1 << 31) != 0;
    }
}
