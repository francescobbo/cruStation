use crate::hw::bus::{BusDevice};
// use crate::hw::cpu::{Cpu, PsxBus};
use crate::hw::vec::ByteSerialized;

use std::fs::File;
use std::io::Read;

pub struct Bios {
    memory: Vec<u8>,
}

impl Bios {
    pub fn new() -> Bios {
        Bios {
            memory: vec![0; 512 * 1024],
        }
    }

    pub fn load(&mut self, file: &mut File) {
        if let Ok(count) = file.read(&mut self.memory) {
            if count != 512 * 1024 {
                println!("BIOS ROM size is not 512 * 1024, proceeding nonetheless");
            }
        } else {
            panic!("Could not read BIOS file");
        }
    }
}

impl BusDevice for Bios {
    fn read<const S: u32>(&mut self, addr: u32) -> u32 {
        self.memory.read::<S>(addr)
    }

    fn write<const S: u32>(&mut self, _addr: u32, _value: u32) {
        panic!("Attempt to write in the BIOS ROM");
    }
}

// impl Bios {
//     pub fn call_a<T: PsxBus>(cpu: &mut Cpu<T>) {
//         match cpu.regs[9] {
//             0x00 => {
//                 println!("FileOpen");
//             }
//             0x01 => {
//                 println!("FileSeek");
//             }
//             0x02 => {
//                 println!("FileRead");
//             }
//             0x03 => {
//                 println!("FileWrite");
//             }
//             0x04 => {
//                 println!("FileClose");
//             }
//             0x05 => {
//                 println!("FileIoctl");
//             }
//             0x06 => {
//                 println!("exit");
//             }
//             0x07 => {
//                 println!("FileGetDeviceFlag");
//             }
//             0x08 => {
//                 println!("FileGetc");
//             }
//             0x09 => {
//                 println!("FilePutc");
//             }
//             0x0A => {
//                 println!("todigit");
//             }
//             0x0B => {
//                 println!("atof");
//             }
//             0x0C => {
//                 println!("strtoul");
//             }
//             0x0D => {
//                 println!("strtol");
//             }
//             0x0E => {
//                 println!("abs");
//             }
//             0x0F => {
//                 println!("labs");
//             }
//             0x10 => {
//                 println!("atoi");
//             }
//             0x11 => {
//                 println!("atol");
//             }
//             0x12 => {
//                 println!("atob");
//             }
//             0x13 => {
//                 println!("SaveState");
//             }
//             0x14 => {
//                 println!("RestoreState");
//             }
//             0x15 => {
//                 println!("strcat");
//             }
//             0x16 => {
//                 println!("strncat");
//             }
//             0x17 => {
//                 println!("strcmp");
//             }
//             0x18 => {
//                 println!("strncmp");
//             }
//             0x19 => {
//                 println!("strcpy");
//             }
//             0x1A => {
//                 println!("strncpy");
//             }
//             0x1B => {
//                 println!("strlen");
//             }
//             0x1C => {
//                 println!("index");
//             }
//             0x1D => {
//                 println!("rindex");
//             }
//             0x1E => {
//                 println!("strchr");
//             }
//             0x1F => {
//                 println!("strrchr");
//             }
//             0x20 => {
//                 println!("strpbrk");
//             }
//             0x21 => {
//                 println!("strspn");
//             }
//             0x22 => {
//                 println!("strcspn");
//             }
//             0x23 => {
//                 println!("strtok");
//             }
//             0x24 => {
//                 println!("strstr");
//             }
//             0x25 => {
//                 println!("toupper");
//             }
//             0x26 => {
//                 println!("tolower");
//             }
//             0x27 => {
//                 println!("bcopy");
//             }
//             0x28 => {
//                 println!("bzero");
//             }
//             0x29 => {
//                 println!("bcmp");
//             }
//             0x2A => {
//                 println!("memcpy");
//             }
//             0x2B => {
//                 println!("memset");
//             }
//             0x2C => {
//                 println!("memmove");
//             }
//             0x2D => {
//                 println!("memcmp");
//             }
//             0x2E => {
//                 println!("memchr");
//             }
//             0x2F => {
//                 println!("rand");
//             }
//             0x30 => {
//                 println!("srand");
//             }
//             0x31 => {
//                 println!("qsort");
//             }
//             0x32 => {
//                 println!("strtod");
//             }
//             0x33 => {
//                 println!("malloc");
//             }
//             0x34 => {
//                 println!("free");
//             }
//             0x35 => {
//                 println!("lsearch");
//             }
//             0x36 => {
//                 println!("bsearch");
//             }
//             0x37 => {
//                 println!("calloc");
//             }
//             0x38 => {
//                 println!("realloc");
//             }
//             0x39 => {
//                 println!("InitHeap");
//             }
//             0x3A => {
//                 println!("SystemErrorExit");
//             }
//             0x3B => {
//                 println!("std_in_getchar");
//             }
//             0x3C => {
//                 print!("{}", cpu.regs[4] as u8 as char);
//             }
//             0x3D => {
//                 println!("std_in_gets");
//             }
//             0x3E => {
//                 println!("std_out_puts");
//             }
//             0x3F => { /*println!("printf");*/ }
//             0x40 => {
//                 panic!("SystemErrorUnresolvedException");
//             }
//             0x41 => {
//                 println!("LoadExeHeader");
//             }
//             0x42 => {
//                 println!("LoadExeFile");
//             }
//             0x43 => {
//                 println!("DoExecute");
//             }
//             0x44 => {
//                 println!("FlushCache");
//             }
//             0x45 => {
//                 println!("init_a0_b0_c0_vectors");
//             }
//             0x46 => {
//                 println!("GPU_dw");
//             }
//             0x47 => {
//                 println!("gpu_send_dma");
//             }
//             0x48 => {
//                 println!("SendGP1Command");
//             }
//             0x49 => {
//                 println!("GPU_cw");
//             }
//             0x4A => {
//                 println!("GPU_cwp");
//             }
//             0x4B => {
//                 println!("send_gpu_linked_list");
//             }
//             0x4C => {
//                 println!("gpu_abort_dma");
//             }
//             0x4D => {
//                 println!("GetGPUStatus");
//             }
//             0x4E => {
//                 println!("gpu_sync");
//             }
//             0x4F => {
//                 println!("SystemError");
//             }
//             0x50 => {
//                 println!("SystemError");
//             }
//             0x51 => {
//                 println!("LoadAndExecute");
//             }
//             0x52 => {
//                 println!("SystemError");
//             }
//             0x53 => {
//                 println!("SystemError");
//             }
//             0x54 => {
//                 println!("CdInit");
//             }
//             0x55 => {
//                 println!("_bu_init");
//             }
//             0x56 => {
//                 println!("CdRemove");
//             }
//             0x57 => {
//                 println!("return");
//             }
//             0x58 => {
//                 println!("return");
//             }
//             0x59 => {
//                 println!("return");
//             }
//             0x5A => {
//                 println!("return");
//             }
//             0x5B => {
//                 println!("dev_tty_init");
//             }
//             0x5C => {
//                 println!("dev_tty_open");
//             }
//             0x5D => {
//                 println!("dev_tty_in_out");
//             }
//             0x5E => {
//                 println!("dev_tty_ioctl");
//             }
//             0x5F => {
//                 println!("dev_cd_open");
//             }
//             0x60 => {
//                 println!("dev_cd_read");
//             }
//             0x61 => {
//                 println!("dev_cd_close");
//             }
//             0x62 => {
//                 println!("dev_cd_firstfile");
//             }
//             0x63 => {
//                 println!("dev_cd_nextfile");
//             }
//             0x64 => {
//                 println!("dev_cd_chdir");
//             }
//             0x65 => {
//                 println!("dev_card_open");
//             }
//             0x66 => {
//                 println!("dev_card_read");
//             }
//             0x67 => {
//                 println!("dev_card_write");
//             }
//             0x68 => {
//                 println!("dev_card_close");
//             }
//             0x69 => {
//                 println!("dev_card_firstfile");
//             }
//             0x6A => {
//                 println!("dev_card_nextfile");
//             }
//             0x6B => {
//                 println!("dev_card_erase");
//             }
//             0x6C => {
//                 println!("dev_card_undelete");
//             }
//             0x6D => {
//                 println!("dev_card_format");
//             }
//             0x6E => {
//                 println!("dev_card_rename");
//             }
//             0x6F => {
//                 println!("card_clear_error?");
//             }
//             0x70 => {
//                 println!("_bu_init");
//             }
//             0x71 => {
//                 println!("CdInit");
//             }
//             0x72 => {
//                 println!("CdRemove");
//             }
//             0x73 => {
//                 println!("return");
//             }
//             0x74 => {
//                 println!("return");
//             }
//             0x75 => {
//                 println!("return");
//             }
//             0x76 => {
//                 println!("return");
//             }
//             0x77 => {
//                 println!("return");
//             }
//             0x78 => {
//                 println!("CdAsyncSeekL");
//             }
//             0x79 => {
//                 println!("return");
//             }
//             0x7A => {
//                 println!("return");
//             }
//             0x7B => {
//                 println!("return");
//             }
//             0x7C => {
//                 println!("CdAsyncGetStatus");
//             }
//             0x7D => {
//                 println!("return");
//             }
//             0x7E => {
//                 println!("CdAsyncReadSector");
//             }
//             0x7F => {
//                 println!("return");
//             }
//             0x80 => {
//                 println!("return");
//             }
//             0x81 => {
//                 println!("CdAsyncSetMode");
//             }
//             0x82 => {
//                 println!("return");
//             }
//             0x83 => {
//                 println!("return");
//             }
//             0x84 => {
//                 println!("return");
//             }
//             0x85 => {
//                 println!("return");
//             }
//             0x86 => {
//                 println!("return");
//             }
//             0x87 => {
//                 println!("return");
//             }
//             0x88 => {
//                 println!("return");
//             }
//             0x89 => {
//                 println!("return");
//             }
//             0x8A => {
//                 println!("return");
//             }
//             0x8B => {
//                 println!("return");
//             }
//             0x8C => {
//                 println!("return");
//             }
//             0x8D => {
//                 println!("return");
//             }
//             0x8E => {
//                 println!("return");
//             }
//             0x8F => {
//                 println!("return");
//             }
//             0x90 => {
//                 println!("CdromIoIrqFunc1");
//             }
//             0x91 => {
//                 println!("CdromDmaIrqFunc1");
//             }
//             0x92 => {
//                 println!("CdromIoIrqFunc2");
//             }
//             0x93 => {
//                 println!("CdromDmaIrqFunc2");
//             }
//             0x94 => {
//                 println!("CdromGetInt5errCode");
//             }
//             0x95 => {
//                 println!("CdInitSubFunc");
//             }
//             0x96 => {
//                 println!("AddCDROMDevice");
//             }
//             0x97 => {
//                 println!("AddMemCardDevice");
//             }
//             0x98 => {
//                 println!("AddDuartTtyDevice");
//             }
//             0x99 => {
//                 println!("AddDummyTtyDevice");
//             }
//             0x9A => {
//                 println!("SystemError");
//             }
//             0x9B => {
//                 println!("SystemError");
//             }
//             0x9C => {
//                 println!("SetConf");
//             }
//             0x9D => {
//                 println!("GetConf");
//             }
//             0x9E => {
//                 println!("SetCdromIrqAutoAbort");
//             }
//             0x9F => {
//                 println!("SetMemSize");
//             }
//             0xA0 => {
//                 println!("WarmBoot");
//             }
//             0xA1 => {
//                 println!("SystemErrorBootOrDiskFailure");
//             }
//             0xA2 => {
//                 println!("EnqueueCdIntr");
//             }
//             0xA3 => {
//                 println!("DequeueCdIntr");
//             }
//             0xA4 => {
//                 println!("CdGetLbn");
//             }
//             0xA5 => {
//                 println!("CdReadSector");
//             }
//             0xA6 => {
//                 println!("CdGetStatus");
//             }
//             0xA7 => {
//                 println!("bu_callback_okay");
//             }
//             0xA8 => {
//                 println!("bu_callback_err_write");
//             }
//             0xA9 => {
//                 println!("bu_callback_err_busy");
//             }
//             0xAA => {
//                 println!("bu_callback_err_eject");
//             }
//             0xAB => {
//                 println!("_card_info");
//             }
//             0xAC => {
//                 println!("_card_async_load_directory");
//             }
//             0xAD => {
//                 println!("set_card_auto_format");
//             }
//             0xAE => {
//                 println!("bu_callback_err_prev_write");
//             }
//             0xAF => {
//                 println!("card_write_test");
//             }
//             0xB0 => {
//                 println!("return");
//             }
//             0xB1 => {
//                 println!("return");
//             }
//             0xB2 => {
//                 println!("ioabort_raw");
//             }
//             0xB3 => {
//                 println!("return");
//             }
//             0xB4 => {
//                 println!("GetSystemInfo");
//             }
//             _ => {
//                 println!("Unhandled syscall A({:02x})", cpu.regs[9] & 0xff);
//             }
//         }
//     }

//     pub fn call_b<T: PsxBus>(cpu: &mut Cpu<T>) {
//         match cpu.regs[9] {
//             0x00 => {
//                 println!("alloc_kernel_memory");
//             }
//             0x01 => {
//                 println!("free_kernel_memory");
//             }
//             0x02 => {
//                 println!("init_timer");
//             }
//             0x03 => {
//                 println!("get_timer");
//             }
//             0x04 => {
//                 println!("enable_timer_irq");
//             }
//             0x05 => {
//                 println!("disable_timer_irq");
//             }
//             0x06 => {
//                 println!("restart_timer");
//             }
//             0x07 => {
//                 println!("DeliverEvent");
//             }
//             0x08 => {
//                 println!("OpenEvent");
//             }
//             0x09 => {
//                 println!("CloseEvent");
//             }
//             0x0A => {
//                 println!("WaitEvent");
//             }
//             0x0B => {
//                 println!("TestEvent");
//             }
//             0x0C => {
//                 println!("EnableEvent");
//             }
//             0x0D => {
//                 println!("DisableEvent");
//             }
//             0x0E => {
//                 println!("OpenThread");
//             }
//             0x0F => {
//                 println!("CloseThread");
//             }
//             0x10 => {
//                 println!("ChangeThread");
//             }
//             0x11 => {
//                 println!("jump_to_00000000h");
//             }
//             0x12 => {
//                 println!("InitPad");
//             }
//             0x13 => {
//                 println!("StartPad");
//             }
//             0x14 => {
//                 println!("StopPad");
//             }
//             0x15 => {
//                 println!("OutdatedPadInitAndStart");
//             }
//             0x16 => {
//                 println!("OutdatedPadGetButtons");
//             }
//             0x17 => { /* println!("ReturnFromException"); */ }
//             0x18 => {
//                 println!("SetDefaultExitFromException");
//             }
//             0x19 => {
//                 println!("SetCustomExitFromException");
//             }
//             0x1A => {
//                 println!("SystemError");
//             }
//             0x1B => {
//                 println!("SystemError");
//             }
//             0x1C => {
//                 println!("SystemError");
//             }
//             0x1D => {
//                 println!("SystemError");
//             }
//             0x1E => {
//                 println!("SystemError");
//             }
//             0x1F => {
//                 println!("SystemError");
//             }
//             0x20 => {
//                 println!("UnDeliverEvent");
//             }
//             0x21 => {
//                 println!("SystemError");
//             }
//             0x22 => {
//                 println!("SystemError");
//             }
//             0x23 => {
//                 println!("SystemError");
//             }
//             0x24 => {
//                 println!("jump_to_00000000h");
//             }
//             0x25 => {
//                 println!("jump_to_00000000h");
//             }
//             0x26 => {
//                 println!("jump_to_00000000h");
//             }
//             0x27 => {
//                 println!("jump_to_00000000h");
//             }
//             0x28 => {
//                 println!("jump_to_00000000h");
//             }
//             0x29 => {
//                 println!("jump_to_00000000h");
//             }
//             0x2A => {
//                 println!("SystemError");
//             }
//             0x2B => {
//                 println!("SystemError");
//             }
//             0x2C => {
//                 println!("jump_to_00000000h");
//             }
//             0x2D => {
//                 println!("jump_to_00000000h");
//             }
//             0x2E => {
//                 println!("jump_to_00000000h");
//             }
//             0x2F => {
//                 println!("jump_to_00000000h");
//             }
//             0x30 => {
//                 println!("jump_to_00000000h");
//             }
//             0x31 => {
//                 println!("jump_to_00000000h");
//             }
//             0x32 => {
//                 println!("FileOpen");
//             }
//             0x33 => {
//                 println!("FileSeek");
//             }
//             0x34 => {
//                 println!("FileRead");
//             }
//             0x35 => {
//                 println!("FileWrite");
//             }
//             0x36 => {
//                 println!("FileClose");
//             }
//             0x37 => {
//                 println!("FileIoctl");
//             }
//             0x38 => {
//                 println!("exit");
//             }
//             0x39 => {
//                 println!("FileGetDeviceFlag");
//             }
//             0x3A => {
//                 println!("FileGetc");
//             }
//             0x3B => {
//                 println!("FilePutc");
//             }
//             0x3C => {
//                 println!("std_in_getchar");
//             }
//             0x3D => {
//                 print!("{}", cpu.regs[4] as u8 as char);
//             }
//             0x3E => {
//                 println!("std_in_gets");
//             }
//             0x3F => {
//                 println!("std_out_puts");
//             }
//             0x40 => {
//                 println!("chdir");
//             }
//             0x41 => {
//                 println!("FormatDevice");
//             }
//             0x42 => {
//                 println!("firstfile");
//             }
//             0x43 => {
//                 println!("nextfile");
//             }
//             0x44 => {
//                 println!("FileRename");
//             }
//             0x45 => {
//                 println!("FileDelete");
//             }
//             0x46 => {
//                 println!("FileUndelete");
//             }
//             0x47 => {
//                 println!("AddDevice");
//             }
//             0x48 => {
//                 println!("RemoveDevice");
//             }
//             0x49 => {
//                 println!("PrintInstalledDevices");
//             }
//             0x4A => {
//                 println!("InitCard");
//             }
//             0x4B => {
//                 println!("StartCard");
//             }
//             0x4C => {
//                 println!("StopCard");
//             }
//             0x4D => {
//                 println!("_card_info_subfunc");
//             }
//             0x4E => {
//                 println!("write_card_sector");
//             }
//             0x4F => {
//                 println!("read_card_sector");
//             }
//             0x50 => {
//                 println!("allow_new_card");
//             }
//             0x51 => {
//                 println!("Krom2RawAdd");
//             }
//             0x52 => {
//                 println!("SystemError");
//             }
//             0x53 => {
//                 println!("Krom2Offset");
//             }
//             0x54 => {
//                 println!("GetLastError");
//             }
//             0x55 => {
//                 println!("GetLastFileError");
//             }
//             0x56 => {
//                 println!("GetC0Table");
//             }
//             0x57 => {
//                 println!("GetB0Table");
//             }
//             0x58 => {
//                 println!("get_bu_callback_port");
//             }
//             0x59 => {
//                 println!("testdevice");
//             }
//             0x5A => {
//                 println!("SystemError");
//             }
//             0x5B => {
//                 println!("ChangeClearPad");
//             }
//             0x5C => {
//                 println!("get_card_status");
//             }
//             0x5D => {
//                 println!("wait_card_status");
//             }
//             0x5E => {
//                 println!("N/A");
//             }
//             _ => {
//                 println!("Unhandled syscall B({:02x})", cpu.regs[9] & 0xff);
//             }
//         }
//     }

//     pub fn call_c<T: PsxBus>(cpu: &mut Cpu<T>) {
//         match cpu.regs[9] & 0x7f {
//             0x00 => {
//                 println!("EnqueueTimerAndVblankIrqs");
//             }
//             0x01 => {
//                 println!("EnqueueSyscallHandler");
//             }
//             0x02 => {
//                 println!("SysEnqIntRP");
//             }
//             0x03 => {
//                 println!("SysDeqIntRP");
//             }
//             0x04 => {
//                 println!("get_free_EvCB_slot");
//             }
//             0x05 => {
//                 println!("get_free_TCB_slot");
//             }
//             0x06 => {
//                 println!("ExceptionHandler");
//             }
//             0x07 => {
//                 println!("InstallExceptionHandlers");
//             }
//             0x08 => {
//                 println!("SysInitMemory");
//             }
//             0x09 => {
//                 println!("SysInitKernelVariables");
//             }
//             0x0A => {
//                 println!("ChangeClearRCnt");
//             }
//             0x0B => {
//                 println!("SystemError");
//             }
//             0x0C => {
//                 println!("InitDefInt");
//             }
//             0x0D => {
//                 println!("SetIrqAutoAck");
//             }
//             0x0E => {
//                 println!("return");
//             }
//             0x0F => {
//                 println!("return");
//             }
//             0x10 => {
//                 println!("return");
//             }
//             0x11 => {
//                 println!("return");
//             }
//             0x12 => {
//                 println!("InstallDevices");
//             }
//             0x13 => {
//                 println!("FlushStdInOutPut");
//             }
//             0x14 => {
//                 println!("return");
//             }
//             0x15 => {
//                 println!("tty_cdevinput");
//             }
//             0x16 => {
//                 println!("tty_cdevscan");
//             }
//             0x17 => {
//                 println!("tty_circgetc");
//             }
//             0x18 => {
//                 println!("tty_circputc");
//             }
//             0x19 => {
//                 println!("ioabort");
//             }
//             0x1A => {
//                 println!("set_card_find_mode");
//             }
//             0x1B => {
//                 println!("KernelRedirect");
//             }
//             0x1C => {
//                 println!("AdjustA0Table");
//             }
//             0x1D => {
//                 println!("get_card_find_mode");
//             }
//             _ => {
//                 println!("Unhandled syscall C({:02x})", cpu.regs[9] & 0x7f);
//             }
//         }
//     }
// }
