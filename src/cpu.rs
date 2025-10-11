use colored::*;

// cpu.rs
use crate::bus::Bus;

use crate::constants::*;

pub struct Cpu {
    regs: [u32; 32],
    pc: u32,
    bus: Bus,
    cycles: u64,
}


const OPCODE_MASK: u32 = 0x7F;

impl Cpu {
    pub fn new(bus: Bus, pc: u32) -> Self {
        Self {
            regs: [0; 32],
            pc,
            bus,
            cycles: 0,
        }
    }
    // Used for debugging
    #[allow(dead_code)]
    pub fn read_mem(&mut self, size: u8, addr: u32) -> u32 {
        match self.bus.read(size, addr){
            Ok(value) => value,
            Err(_) => panic!("Memory read error at address 0x{:08X}", addr),
        }
    }
    /// Used for debugging
    pub fn read_reg(&self, reg: usize) -> u32 {
        if reg == 0 {
            0
        } else {
            self.regs[reg]
        }
    }
    // Not very usefull, using it to avoid writting to x0.
    // can probably also be used for later register modifications.
    fn write_reg(&mut self, reg: usize, value: u32) {
        if reg != 0 {
            self.regs[reg] = value;
        }
    }

    // probably better way to do this
    fn sign_extend(&self, value: u32, bits: u32) -> u32 {
        let shift = 32 - bits;
        (((value << shift) as i32) >> shift) as u32
    }

    pub fn tick(&mut self, verbose: bool, batch: u64) {
        for _ in 0..batch {
            assert!(
                self.pc.is_multiple_of(4),
                "{}",
                format!("PC not aligned: 0x{:08X}", self.pc).red().bold()
            );
            let instruction = match self.bus.read(4, self.pc) {
                Ok(value) => value,
                Err(_) => {
                    panic!("{}", format!("Cycle: {} Memory read error at PC: 0x{:08X}", self.cycles, self.pc).red().bold());
                },
            };

            // decoding constants
            let _opcode = instruction & OPCODE_MASK;
            let _rd = (instruction >> 7) & 0x1F;
            let funct3 = (instruction >> 12) & 0x7;
            let rs1 = (instruction >> 15) & 0x1F;
            let rs2 = (instruction >> 20) & 0x1F;
            let funct7 = (instruction >> 25) & 0x7F;

            let mnemonic: String;
            let mut pc_changed = false;
            match _opcode {
                R_FORMAT => {
                    match funct3 {
                        FUNCT3_ADD_SUB => {
                            if funct7 == FUNCT7_ADD {
                                mnemonic = format!("add {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                                self.regs[_rd as usize] = self.read_reg(rs1 as usize).wrapping_add(self.read_reg(rs2 as usize));
                            } else {
                                mnemonic = format!("sub {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                                self.regs[_rd as usize] = self.read_reg(rs1 as usize).wrapping_sub(self.read_reg(rs2 as usize));
                            }
                        }
                        FUNCT3_XOR => {
                            mnemonic = format!("xor {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                            self.regs[_rd as usize] = self.read_reg(rs1 as usize) ^ self.read_reg(rs2 as usize);
                        }
                        FUNCT3_OR => {
                            mnemonic = format!("or {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                            self.regs[_rd as usize] = self.read_reg(rs1 as usize) | self.read_reg(rs2 as usize);
                        }
                        FUNCT3_AND => {
                            mnemonic = format!("and {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                            self.regs[_rd as usize] = self.read_reg(rs1 as usize) & self.read_reg(rs2 as usize);
                        }
                        FUNCT3_SLL => {
                            mnemonic = format!("sll {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                            let shamt = self.read_reg(rs2 as usize) & 0x1F;
                            self.regs[_rd as usize] = self.read_reg(rs1 as usize) << shamt;
                        }
                        FUNCT3_SRL => {
                            if funct7 == FUNCT7_SRL {
                                mnemonic = format!("srl {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                                let shamt = self.read_reg(rs2 as usize) & 0x1F;
                                self.regs[_rd as usize] = self.read_reg(rs1 as usize) >> shamt;
                            } else {
                                mnemonic = format!("sra {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                                let shamt = self.read_reg(rs2 as usize) & 0x1F;
                                self.regs[_rd as usize] = ((self.read_reg(rs1 as usize) as i32) >> shamt) as u32;
                            }
                        }
                        FUNCT3_SLT => {
                            mnemonic = format!("slt {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                            self.regs[_rd as usize] = if (self.read_reg(rs1 as usize) as i32) < (self.read_reg(rs2 as usize) as i32) { 1 } else { 0 };
                        }
                        FUNCT3_SLTU => {
                            mnemonic = format!("sltu {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], rs2);
                            self.regs[_rd as usize] = if self.read_reg(rs1 as usize) < self.read_reg(rs2 as usize) { 1 } else { 0 };
                        }
                        _ => {
                            panic!("Unknown funct3 in R-format: 0b{:03b}", funct3);
                        }
                    }
                }
                I_COMP_FORMAT => {
                    let imm = self.sign_extend((instruction >> 20) & 0xFFF, 12);
                    match funct3 {
                        FUNCT3_ADDI => {
                            let temp = self.read_reg(rs1 as usize).wrapping_add(imm);
                            self.write_reg(_rd as usize, temp);
                            mnemonic = format!("addi {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], imm as i32);
                        }
                        FUNCT3_XORI => {
                            let temp = self.read_reg(rs1 as usize) ^ imm;
                            self.write_reg(_rd as usize, temp);
                            mnemonic = format!("xori {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], imm as i32);
                        }
                        FUNCT3_ORI => {
                            let temp = self.read_reg(rs1 as usize) | imm;
                            self.write_reg(_rd as usize, temp);
                            mnemonic = format!("ori {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], imm as i32);
                        }
                        FUNCT3_ANDI => {
                            let temp = self.read_reg(rs1 as usize) & imm;
                            self.write_reg(_rd as usize, temp);
                            mnemonic = format!("andi {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], imm as i32);
                        }
                        FUNCT3_SLLI => {
                            let shamt = (instruction >> 20) & 0x1F;
                            let temp = self.read_reg(rs1 as usize) << shamt;
                            self.write_reg(_rd as usize, temp);
                            mnemonic = format!("slli {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], shamt);
                        }
                        FUNCT3_SRLI => {
                            if funct7 == 0x00 {
                                let shamt = (instruction >> 20) & 0x1F;
                                let temp = self.read_reg(rs1 as usize) >> shamt;
                                self.write_reg(_rd as usize, temp);
                                mnemonic = format!("srli {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], shamt);
                            } else {
                                let shamt = (instruction >> 20) & 0x1F;
                                let temp = ((self.read_reg(rs1 as usize) as i32) >> shamt) as u32;
                                self.write_reg(_rd as usize, temp);
                                mnemonic = format!("srai {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], shamt);
                            }
                        }
                        FUNCT3_SLTI => {
                            let imm = ((instruction as i32) >> 20) as u32; // sign-extend
                            let temp = if (self.read_reg(rs1 as usize) as i32) < (imm as i32) { 1 } else { 0 };
                            self.write_reg(_rd as usize, temp);
                            mnemonic = format!("slti {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], imm as i32);
                        }
                        FUNCT3_SLTIU => {
                            let imm = ((instruction as i32) >> 20) as u32; // sign-extend
                            let temp = if self.read_reg(rs1 as usize) < imm { 1 } else { 0 };
                            self.write_reg(_rd as usize, temp);
                            mnemonic = format!("sltiu {}, {}, {}", REGISTER_NAMES[_rd as usize], REGISTER_NAMES[rs1 as usize], imm as i32);
                        }
                        _ => {
                            panic!("Unknown funct3 in I-COMP-format: 0b{:03b}", funct3);
                        }
                    }
                }
                I_LOAD_FORMAT => {
                    let imm = self.sign_extend((instruction >> 20) & 0xFFF, 12);
                    match funct3 {
                        FUNCT3_LB => {
                            let addr = self.read_reg(rs1 as usize).wrapping_add(imm);
                            mnemonic = format!("lb {}, {}({})", REGISTER_NAMES[_rd as usize],  imm as i32, REGISTER_NAMES[rs1 as usize]);
                            match self.bus.read(1, addr) {
                                Ok(byte) => {
                                    let temp = ((byte as i8) as i32) as u32; // sign-extend
                                    self.write_reg(_rd as usize, temp);
                                },
                                Err(_) => {
                                    panic!("Cycle: {} Memory read error at address: 0x{:08X} from {}", self.cycles, addr, mnemonic.bold().underline());
                                }
                            }
                        }
                        FUNCT3_LH => {
                            let addr = self.read_reg(rs1 as usize).wrapping_add(imm);
                            mnemonic = format!("lh {}, {}({})", REGISTER_NAMES[_rd as usize],  imm as i32, REGISTER_NAMES[rs1 as usize]);
                            match self.bus.read(2, addr) {
                                Ok(halfword) => {
                                    let temp = ((halfword as i16) as i32) as u32; // sign-extend
                                    self.write_reg(_rd as usize, temp);
                                },
                                Err(_) => {
                                    panic!("Cycle: {} Memory read error at address: 0x{:08X} from {}", self.cycles, addr, mnemonic.bold().underline());
                                }
                            }
                        }
                        FUNCT3_LW => {
                            let addr = self.read_reg(rs1 as usize).wrapping_add(imm);
                            mnemonic = format!("lw {}, {}({})", REGISTER_NAMES[_rd as usize],  imm as i32, REGISTER_NAMES[rs1 as usize]);
                            match self.bus.read(4, addr) {
                                Ok(word) => {
                                    self.write_reg(_rd as usize, word);
                                },
                                Err(_) => {
                                    panic!("Cycle: {} Memory read error at address: 0x{:08X} from {}", self.cycles, addr, mnemonic.bold().underline());
                                }
                            }
                        }
                        FUNCT3_LBU => {
                            let addr = self.read_reg(rs1 as usize).wrapping_add(imm);
                            mnemonic = format!("lbu {}, {}({})", REGISTER_NAMES[_rd as usize],  imm as i32, REGISTER_NAMES[rs1 as usize]);
                            match self.bus.read(1, addr) {
                                Ok(byte) => {
                                    let temp = byte; // zero-extend
                                    self.write_reg(_rd as usize, temp);
                                },
                                Err(_) => {
                                    panic!("Cycle: {} Memory read error at address: 0x{:08X} from {}", self.cycles, addr, mnemonic.bold().underline());
                                }
                            }
                        }
                        FUNCT3_LHU => {
                            let addr = self.read_reg(rs1 as usize).wrapping_add(imm);
                            mnemonic = format!("lhu {}, {}({})", REGISTER_NAMES[_rd as usize],  imm as i32, REGISTER_NAMES[rs1 as usize]);
                            match self.bus.read(2, addr) {
                                Ok(halfword) => {
                                    let temp = halfword; // zero-extend
                                    self.write_reg(_rd as usize, temp);
                                },
                                Err(_) => {
                                    panic!("Cycle: {} Memory read error at address: 0x{:08X} from {}", self.cycles, addr, mnemonic.bold().underline());
                                }
                            }
                        }
                        _ => {
                            panic!("Unknown funct3 in I-LOAD-format: 0b{:03b}", funct3);
                        }
                    }
                }
                S_FORMAT => {
                    let imm_4_0 = (instruction >> 7) & 0x1F;
                    let imm_11_5 = (instruction >> 25) & 0x7F;
                    let imm = self.sign_extend((imm_11_5 << 5) | imm_4_0, 12);
                    match funct3 {
                        FUNCT3_SB => {
                            let addr = self.read_reg(rs1 as usize).wrapping_add(imm);
                            let value = self.read_reg(rs2 as usize) & 0xFF;
                            mnemonic = format!("sb {}, {}({})", REGISTER_NAMES[rs2 as usize], imm, REGISTER_NAMES[rs1 as usize]);
                            match self.bus.write(1, addr, value) {
                                Ok(_) => {},
                                Err(_) => {
                                    panic!("Cycle: {} Memory write error at address: 0x{:08X} from {}", self.cycles, addr, mnemonic.bold().underline());
                                }
                            }
                        }
                        FUNCT3_SH => {
                            let addr = self.read_reg(rs1 as usize).wrapping_add(imm);
                            let value = self.read_reg(rs2 as usize) & 0xFFFF;
                            mnemonic = format!("sh {}, {}({})", REGISTER_NAMES[rs2 as usize], imm, REGISTER_NAMES[rs1 as usize]);
                            match self.bus.write(2, addr, value) {
                                Ok(_) => {},
                                Err(_) => {
                                    panic!("Cycle: {} Memory write error at address: 0x{:08X} from {}", self.cycles, addr, mnemonic.bold().underline());
                                }
                            }
                        }
                        FUNCT3_SW => {
                            let addr = self.read_reg(rs1 as usize).wrapping_add(imm);
                            let value = self.read_reg(rs2 as usize);
                            mnemonic = format!("sw {}, {}({})", REGISTER_NAMES[rs2 as usize], imm, REGISTER_NAMES[rs1 as usize]);
                            match self.bus.write(4, addr, value) {
                                Ok(_) => {},
                                Err(_) => {
                                    panic!("Cycle: {} Memory write error at address: 0x{:08X} from {}", self.cycles, addr, mnemonic.bold().underline());
                                }
                            }
                        }
                        _ => {
                            panic!("Unknown funct3 in S-format: 0b{:03b}", funct3);
                        }
                    }
                }
                B_FORMAT => {
                    pc_changed = true;
                    let imm_11 = (instruction >> 7) & 0x1;
                    let imm_4_1 = (instruction >> 8) & 0xF;
                    let imm_10_5 = (instruction >> 25) & 0x3F;
                    let imm_12 = (instruction >> 31) & 0x1;
                    let imm = self.sign_extend((imm_12 << 12) | (imm_11 << 11) | (imm_10_5 << 5) | (imm_4_1 << 1), 13);
                    match funct3 {
                        FUNCT3_BEQ => {
                            let address = self.pc.wrapping_add(imm);
                            mnemonic = format!("beq {} , {}, to 0x{:08X}", rs1, rs2, address);
                            if self.read_reg(rs1 as usize) == self.read_reg(rs2 as usize) {
                                self.pc = address;
                            } else {
                                pc_changed = false;
                            }
                        }
                        FUNCT3_BNE => {
                            let address = self.pc.wrapping_add(imm);
                            mnemonic = format!("bne {} , {}, to 0x{:08X}", rs1, rs2, address);
                            if self.read_reg(rs1 as usize) != self.read_reg(rs2 as usize) {
                                self.pc = address;
                            } else {
                                pc_changed = false;
                            }
                        }
                        FUNCT3_BLT =>{
                            let address = self.pc.wrapping_add(imm);
                            mnemonic = format!("blt {} , {}, to 0x{:08X}", rs1, rs2, address);
                            if (self.read_reg(rs1 as usize) as i32) < (self.read_reg(rs2 as usize) as i32) {
                                self.pc = address;
                            } else {
                                pc_changed = false;
                            }
                        }
                        FUNCT3_BGE =>{
                            let address = self.pc.wrapping_add(imm);
                            mnemonic = format!("bge {} , {}, to 0x{:08X}", rs1, rs2, address);
                            if (self.read_reg(rs1 as usize) as i32) >= (self.read_reg(rs2 as usize) as i32) {
                                self.pc = address;
                            } else {
                                pc_changed = false;
                            }
                        }
                        FUNCT3_BLTU =>{
                            let address = self.pc.wrapping_add(imm);
                            mnemonic = format!("bltu {} , {}, to 0x{:08X}", rs1, rs2, address);
                            if self.read_reg(rs1 as usize) < self.read_reg(rs2 as usize) {
                                self.pc = address;
                            } else {
                                pc_changed = false;
                            }
                        }
                        FUNCT3_BGEU =>{
                            let address = self.pc.wrapping_add(imm);
                            mnemonic = format!("bgeu {} , {}, to 0x{:08X}", rs1, rs2, address);
                            if self.read_reg(rs1 as usize) >= self.read_reg(rs2 as usize) {
                                self.pc = address;
                            } else {
                                pc_changed = false;
                            }
                        }
                        _=> {
                            panic!("Unknown funct3 in B-format: 0b{:03b}", funct3);
                        }
                    }
                }
                U_FORMAT_LUI => {
                    mnemonic = "lui".to_string();
                    let imm = instruction & 0xFFFFF000;
                    self.write_reg(_rd as usize, imm);
                }
                U_FORMAT_AUIPC => {
                    mnemonic = "auipc".to_string();
                    let imm = instruction & 0xFFFFF000;
                    let temp = self.pc.wrapping_add(imm);
                    self.write_reg(_rd as usize, temp);
                }
                J_FORMAT => {
                    pc_changed = true;
                    let imm20 = (instruction >> 31) & 0x1;
                    let imm10_1 = (instruction >> 21) & 0x3FF;
                    let imm11_1 = (instruction >> 20) & 0x1;
                    let imm19_12 = (instruction >> 12) & 0xFF;
                    let imm = self.sign_extend((imm20 << 20) | (imm19_12 << 12) | (imm11_1 << 11) | (imm10_1 << 1), 21);
                    let addr = self.pc.wrapping_add(imm);
                    self.write_reg(_rd as usize, self.pc.wrapping_add(4));
                    self.pc = addr;
                    mnemonic = format!("jal to 0x{addr:08X}");
                }
                I_JALR_FORMAT => {
                    pc_changed = true;
                    mnemonic = "jalr".to_string();
                    let imm = self.sign_extend((instruction >> 20) & 0xFFF, 12);
                    let addr = self.read_reg(rs1 as usize).wrapping_add(imm) & !1;
                    self.write_reg(_rd as usize, self.pc.wrapping_add(4));
                    self.pc = addr;
                }
                I_ENV_FORMAT => {
                    // mnemonic = "ecall/ebreak".to_string();
                    unimplemented!("ECALL/EBREAK not implemented");
                }
                0x0 => {
                    mnemonic = "NOP".to_string();
                    // Do nothing
                }
                _ => {
                    panic!("Unknown opcode: 0x{:02X}", _opcode);
                }
            }
            if verbose {
                println!("{}: {} (0x{:08X}) pc: 0x{:08X}", self.cycles, mnemonic.to_string().bold().underline(), instruction, self.pc);
            }
            if !pc_changed {
                self.pc += 4;
            }
            self.cycles += 1;
        }
    }
}
