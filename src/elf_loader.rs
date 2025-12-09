// elf_loader.rs

// ChatGPT helped port the elf loader from C to Rust. Care should be taken 
// to ensure correctness and safety.
#![allow(dead_code)]

use crate::bus::Bus;

type Result<T> = std::result::Result<T, String>;

// ---- ELF constants (ELF32 / little-endian) ----
const EI_NIDENT: usize = 16;

const EI_MAG0: usize = 0;
const EI_MAG1: usize = 1;
const EI_MAG2: usize = 2;
const EI_MAG3: usize = 3;
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const EI_OSABI: usize = 7;
const EI_ABIVERSION: usize = 8;

const ELFMAG0: u8 = 0x7F;
const ELFMAG1: u8 = b'E';
const ELFMAG2: u8 = b'L';
const ELFMAG3: u8 = b'F';

const ELFCLASS32: u8 = 1;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;

const ET_EXEC: u16 = 2;
const EM_RISCV: u16 = 243;

const PT_LOAD: u32 = 1;
const SHT_NOBITS: u32 = 8;

// ---- ELF header structs ----
#[derive(Debug, Clone)]
pub struct Elf32Ehdr {
    e_ident: [u8; EI_NIDENT],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u32,
    e_phoff: u32,
    e_shoff: u32,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[derive(Debug, Clone)]
pub struct Elf32Phdr {
    p_type: u32,
    p_offset: u32,
    p_vaddr: u32,
    p_paddr: u32,
    p_filesz: u32,
    p_memsz: u32,
    p_flags: u32,
    p_align: u32,
}

#[derive(Debug, Clone)]
pub struct Elf32Shdr {
    sh_name: u32,
    sh_type: u32,
    sh_flags: u32,
    sh_addr: u32,
    sh_offset: u32,
    sh_size: u32,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: u32,
    sh_entsize: u32,
}

// ---- Safe readers from Vec<u32> by BYTE offset ----
// Assumes each u32 in elf_vec is a LE word read from the file.
#[inline]
fn check_bounds(words: &[u32], off: usize, len: usize) -> Result<()> {
    let total = words.len() * 4;
    if off + len > total {
        Err("ELF: out-of-bounds access".into())
    } else {
        Ok(())
    }
}

#[inline]
fn rd_u8(words: &[u32], off: usize) -> Result<u8> {
    check_bounds(words, off, 1)?;
    let w = words[off / 4].to_le_bytes();
    Ok(w[off % 4])
}

#[inline]
fn rd_u16_le(words: &[u32], off: usize) -> Result<u16> {
    Ok(u16::from_le_bytes([rd_u8(words, off)?, rd_u8(words, off + 1)?]))
}

#[inline]
fn rd_u32_le(words: &[u32], off: usize) -> Result<u32> {
    Ok(u32::from_le_bytes([
        rd_u8(words, off)?,
        rd_u8(words, off + 1)?,
        rd_u8(words, off + 2)?,
        rd_u8(words, off + 3)?,
    ]))
}

// ---- Parsers ----
pub fn read_elf_header(elf_vec: &[u32]) -> Result<Elf32Ehdr> {
    // ELF32 header is 52 bytes
    check_bounds(elf_vec, 0, 52)?;

    let mut e_ident = [0u8; EI_NIDENT];
    for (i, item) in e_ident.iter_mut().enumerate().take(EI_NIDENT) {
        *item = rd_u8(elf_vec, i)?;
    }
    let mut off = EI_NIDENT;

    let e_type      = rd_u16_le(elf_vec, off)?; off += 2;
    let e_machine   = rd_u16_le(elf_vec, off)?; off += 2;
    let e_version   = rd_u32_le(elf_vec, off)?; off += 4;
    let e_entry     = rd_u32_le(elf_vec, off)?; off += 4;
    let e_phoff     = rd_u32_le(elf_vec, off)?; off += 4;
    let e_shoff     = rd_u32_le(elf_vec, off)?; off += 4;
    let e_flags     = rd_u32_le(elf_vec, off)?; off += 4;
    let e_ehsize    = rd_u16_le(elf_vec, off)?; off += 2;
    let e_phentsize = rd_u16_le(elf_vec, off)?; off += 2;
    let e_phnum     = rd_u16_le(elf_vec, off)?; off += 2;
    let e_shentsize = rd_u16_le(elf_vec, off)?; off += 2;
    let e_shnum     = rd_u16_le(elf_vec, off)?; off += 2;
    let e_shstrndx  = rd_u16_le(elf_vec, off)?; // off += 2;

    Ok(Elf32Ehdr {
        e_ident,
        e_type,
        e_machine,
        e_version,
        e_entry,
        e_phoff,
        e_shoff,
        e_flags,
        e_ehsize,
        e_phentsize,
        e_phnum,
        e_shentsize,
        e_shnum,
        e_shstrndx,
    })
}

fn read_phdr(elf_vec: &[u32], base_off: usize) -> Result<Elf32Phdr> {
    // Elf32_Phdr is 32 bytes
    check_bounds(elf_vec, base_off, 32)?;
    Ok(Elf32Phdr {
        p_type:   rd_u32_le(elf_vec, base_off)?,
        p_offset: rd_u32_le(elf_vec, base_off +  4)?,
        p_vaddr:  rd_u32_le(elf_vec, base_off +  8)?,
        p_paddr:  rd_u32_le(elf_vec, base_off + 12)?,
        p_filesz: rd_u32_le(elf_vec, base_off + 16)?,
        p_memsz:  rd_u32_le(elf_vec, base_off + 20)?,
        p_flags:  rd_u32_le(elf_vec, base_off + 24)?,
        p_align:  rd_u32_le(elf_vec, base_off + 28)?,
    })
}

fn read_shdr(elf_vec: &[u32], base_off: usize) -> Result<Elf32Shdr> {
    // Elf32_Shdr is 40 bytes
    check_bounds(elf_vec, base_off, 40)?;
    Ok(Elf32Shdr {
        sh_name:      rd_u32_le(elf_vec, base_off)?,
        sh_type:      rd_u32_le(elf_vec, base_off +  4)?,
        sh_flags:     rd_u32_le(elf_vec, base_off +  8)?,
        sh_addr:      rd_u32_le(elf_vec, base_off + 12)?,
        sh_offset:    rd_u32_le(elf_vec, base_off + 16)?,
        sh_size:      rd_u32_le(elf_vec, base_off + 20)?,
        sh_link:      rd_u32_le(elf_vec, base_off + 24)?,
        sh_info:      rd_u32_le(elf_vec, base_off + 28)?,
        sh_addralign: rd_u32_le(elf_vec, base_off + 32)?,
        sh_entsize:   rd_u32_le(elf_vec, base_off + 36)?,
    })
}

// ---- Public loader: takes Vec<u32> file image, writes via Bus ----
pub fn elf_loader(bus: &mut Bus, elf_vec: Vec<u32>) -> Result<u32> {
    println!("Loading ELF file...");
    let header = read_elf_header(&elf_vec)?;

    // ---- Validate like in your C code ----
    let id = &header.e_ident;
    if id[EI_MAG0] != ELFMAG0 || id[EI_MAG1] != ELFMAG1 || id[EI_MAG2] != ELFMAG2 || id[EI_MAG3] != ELFMAG3 {
        return Err("Bad ELF magic".into());
    }
    if id[EI_CLASS] != ELFCLASS32           { return Err("Not ELFCLASS32".into()); }
    if id[EI_DATA]  != ELFDATA2LSB          { return Err("Not little-endian ELF".into()); }
    if id[EI_VERSION] != EV_CURRENT         { return Err("Bad ident version".into()); }
    // OSABI/ABIVERSION checks are often relaxed; add if you want strictness
    if header.e_type != ET_EXEC              { return Err("Not ET_EXEC".into()); }
    if header.e_machine != EM_RISCV          { return Err("Not EM_RISCV".into()); }
    if header.e_version != EV_CURRENT as u32 { return Err("Bad e_version".into()); }
    if header.e_ehsize as usize  != 52       { return Err("Unexpected e_ehsize".into()); }
    if header.e_phentsize as usize != 32     { return Err("Unexpected e_phentsize".into()); }
    if header.e_shnum > 0 && header.e_shentsize as usize != 40 {
        return Err("Unexpected e_shentsize".into());
    }

    println!(
        "{} total program headers. Entry: 0x{:08X}",
        header.e_phnum, header.e_entry
    );

    // ---- Load PT_LOAD segments ----
    for i in 0..header.e_phnum {
        let ph_off = header.e_phoff as usize + (i as usize) * (header.e_phentsize as usize);
        let ph = read_phdr(&elf_vec, ph_off)?;

        if ph.p_type != PT_LOAD {
            // println!("PHDR {}: skipped (type={})", i, ph.p_type);
            continue;
        }
        if ph.p_filesz == 0 {
            // println!("PHDR {}: filesz=0 (skip copy)", i);
            // still may want to zero p_memsz, done below
        }

        // Copy [p_offset .. p_offset + p_filesz) -> [p_vaddr .. p_vaddr + p_filesz)
        if ph.p_filesz > 0 {
            let src_start = ph.p_offset as usize;
            // let src_end   = src_start + ph.p_filesz as usize;
            check_bounds(&elf_vec, src_start, ph.p_filesz as usize)?;

            println!(
                "{}: writing RAM @0x{:08X}, off 0x{:08X}, size {}B",
                i, ph.p_vaddr, ph.p_offset, ph.p_filesz
            );

            for j in 0..(ph.p_filesz as usize) {
                let b = rd_u8(&elf_vec, src_start + j)?;
                let _ = bus.write(1, ph.p_vaddr + j as u32, b as u32);
            }
        }

        // If memsz > filesz, zero the tail (common for .bss inside a loadable segment)
        if ph.p_memsz > ph.p_filesz {
            let zero_len = (ph.p_memsz - ph.p_filesz) as usize;
            let start = ph.p_vaddr + ph.p_filesz;
            for j in 0..zero_len {
                let _ = bus.write(1, start + j as u32, 0);
            }
        }
    }

    // ---- Zero explicit BSS sections (SHT_NOBITS) ----
    println!("{} total sections", header.e_shnum);
    for i in 0..header.e_shnum {
        let sh_off = header.e_shoff as usize + (i as usize) * (header.e_shentsize as usize);
        let sh = read_shdr(&elf_vec, sh_off)?;

        if sh.sh_type != SHT_NOBITS || sh.sh_size == 0 {
            // print a small marker if you want:
            // print!("S");
            continue;
        }

        println!(
            "Clearing BSS @0x{:08X}, size: {}B",
            sh.sh_addr, sh.sh_size
        );
        for j in 0..(sh.sh_size as usize) {
            let _ = bus.write(1, sh.sh_addr + j as u32, 0);
        }
    }

    println!("ELF load complete.");
    Ok(header.e_entry)
}
