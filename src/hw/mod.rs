pub mod bios;
pub mod cdrom;
pub mod dma;
pub mod gpu;
pub mod joy_mc;
pub mod ram;
pub mod spu;
pub mod timers;
pub mod vec;

use crate::hw::bios::Bios;
use crate::hw::cdrom::Cdrom;
use crate::hw::dma::Dma;
use crate::hw::gpu::Gpu;
use crate::hw::joy_mc::JoypadMemorycard;
use crate::hw::ram::Ram;
use crate::hw::spu::Spu;
use crate::hw::timers::Timers;
