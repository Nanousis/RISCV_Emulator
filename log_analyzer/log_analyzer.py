from enum import IntEnum
from dataclasses import dataclass
from typing import BinaryIO, Iterator, Optional
import argparse

from capstone import *
from dataclasses import dataclass
from typing import Union


PC_SIZE     = 4 
OPCODE_SIZE = 4 
ADDR_SIZE   = 4 
VALUE_SIZE  = 4 
HEADER_SIZE = 0 
RED_BOLD = "\033[1;31m"
BOLD = "\033[1m"
UNDERLINE = "\033[4m"
RESET = "\033[0m"
OPCODE_MASK = 0x7f
class EventTag(IntEnum):
    RegWrite   = 0
    MemRead    = 1
    MemWrite   = 2
    FlowChange = 3
    FlowLink   = 4

def _read_exact(fp: BinaryIO, n: int) -> bytes:
    b = fp.read(n)
    if b is None or len(b) != n:
        raise EOFError("Unexpected EOF")
    return b

def _read_u8(fp: BinaryIO) -> int:
    return _read_exact(fp, 1)[0]

def _read_uint_le(fp: BinaryIO, nbytes: int) -> int:
    return int.from_bytes(_read_exact(fp, nbytes), "little", signed=False)

def _skip_header(fp: BinaryIO, n: int = HEADER_SIZE) -> None:
    # Try fast seek; fall back to reading/discarding
    try:
        cur = fp.tell()
        fp.seek(cur + n)
    except (OSError, AttributeError):
        _read_exact(fp, n)


@dataclass
class RegWrite:
    reg: int
    value: int

@dataclass
class MemRead:
    addr: int
    value: int

@dataclass
class MemWrite:
    addr: int
    value: int

@dataclass
class FlowChange:
    new_pc: int

@dataclass
class FlowLink:
    new_pc: int
    register: int

# --- Enum type (Rust's EventType) ---

EventType = Union[RegWrite, MemRead, MemWrite, FlowChange, FlowLink]

# --- Struct (Rust's Event) ---

@dataclass
class Event:
    pc: int
    opcode: int
    instr_type: EventType
    str_info: str
    def __str__(self):
        pc_str = f"0x{self.pc:08X}"
        op_str = f"0x{self.opcode:08X}"
        t = self.instr_type

        match t:
            case RegWrite(reg, value):
                return f"[{pc_str}] RegWrite  x{reg} = 0x{value:08X}"
            case MemRead(addr, value):
                return f"[{pc_str}] MemRead   [{addr:#010x}] -> 0x{value:08X}"
            case MemWrite(addr, value):
                return f"[{pc_str}] MemWrite  [{addr:#010x}] = 0x{value:08X}"
            case FlowChange(new_pc):
                return f"[{pc_str}] FlowChange -> 0x{new_pc:08X}"
            case FlowLink(new_pc, register):
                return f"[{pc_str}] FlowLink   new_pc=0x{new_pc:08X}, reg=x{register}"
            case _:
                return f"[{pc_str}] Unknown Event"

emu_count = 0
rtl_count = 0
def parse_next(fp: BinaryIO, md, type, skip_jumps=True):
    while True:
        cnt = 0
        if(type == "emu"):
            global emu_count
            emu_count += 1
            cnt = emu_count
        elif(type == "rtl"):
            global rtl_count
            rtl_count += 1
            cnt = rtl_count
        try:
            pc     = _read_uint_le(fp, PC_SIZE)
            opcode = _read_uint_le(fp, OPCODE_SIZE)
            tag    = EventTag(_read_u8(fp))
        except EOFError:
            return None
        except ValueError as e:
            raise RuntimeError(f"Corrupt stream or wrong sizes? {e}")
        str = ""
        if tag == EventTag.RegWrite:
            reg   = _read_u8(fp)
            value = _read_uint_le(fp, VALUE_SIZE)
            str = f"RegWrite: PC={to_hex(pc)}, Opcode={to_hex(opcode)}, Reg={to_hex(reg)}, Value={to_hex(value)}"
            event = Event(pc, opcode, RegWrite(reg, value),str)
        elif tag in (EventTag.MemRead, EventTag.MemWrite):
            addr  = _read_uint_le(fp, ADDR_SIZE)
            value = _read_uint_le(fp, VALUE_SIZE)
            str = f"{'MemRead' if tag == EventTag.MemRead else 'MemWrite'}: PC={to_hex(pc)}, Opcode={to_hex(opcode)}, Addr={to_hex(addr)}, Value={to_hex(value)}"
            event = Event(pc, opcode, MemRead(addr, value) if tag == EventTag.MemRead else MemWrite(addr, value),str)
        elif tag == EventTag.FlowChange:
            new_pc = _read_uint_le(fp, PC_SIZE)
            str = f"FlowChange: PC={to_hex(pc)}, Opcode={to_hex(opcode)}, NewPC={to_hex(0)}"
            # For the time being ignore new pc
            event = Event(pc, opcode, FlowChange(0),str)
            # event = Event(pc, opcode, FlowChange(new_pc),str)
        elif tag == EventTag.FlowLink:
            new_pc   = _read_uint_le(fp, PC_SIZE)
            register = _read_u8(fp)
            str = f"FlowLink: PC={to_hex(pc)}, Opcode={to_hex(opcode)}, NewPC={to_hex(new_pc)}, Reg={to_hex(register)}"
            event = Event(pc, opcode, FlowLink(new_pc, register),str)
        else:
            raise RuntimeError(f"Unknown tag {tag}")
        # print("Decoding Instruction:", to_hex(pc), to_hex(opcode), to_hex(tag))
        insn = next(md.disasm(opcode.to_bytes(4, byteorder='little'), pc))
        _opcode = opcode & OPCODE_MASK
        str = f"{cnt}: {insn.mnemonic} {insn.op_str} \t + " + str
        print(str)
        # jump instructions are super buggy in the RTL trace, so skip them
        if skip_jumps and _opcode == 0x67:  # JAL, JALR, BRANCH
            continue
        return event

#  8000 0000 0020 0117 0000 0000 0200 0000 

def to_hex(val, width=8):
    """Convert int or None to 0x-prefixed hex string."""
    if val is None:
        return None
    return f"0x{val:0{width}X}"

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Parse a RISCY trace file")
    parser.add_argument("path_emu", help="path to trace file")
    parser.add_argument("path_rtl", help="path to trace file")

    args = parser.parse_args()
    fp_emu = open(args.path_emu, "rb")
    fp_rtl = open(args.path_rtl, "rb")
    mode = CS_MODE_RISCV32 | CS_MODE_RISCVC  # add RISCVC if compressed possible
    md = Cs(CS_ARCH_RISCV, mode)
    i = 0

    mistakes = 0
    while True:
        print(f"{UNDERLINE}{BOLD}--- Instruction {i} ---{RESET}")
        emu_event = parse_next(fp_emu, md, "emu")
        rtl_event = parse_next(fp_rtl, md, "rtl")
        if emu_event is None or rtl_event is None:
            print("End of file reached.")
            break

        if emu_event != rtl_event:
            print(f"{RED_BOLD}Mismatch detected!{RESET}")
            print(f"EMU: {emu_event.str_info}")
            print(f"RTL: {rtl_event.str_info}")
            print(f"{RED_BOLD}------------------{RESET}")
            mistakes += 1
        if mistakes >= 2:
            print(f"{RED_BOLD}Too many mismatches, stopping analysis.{RESET}")
            break
        i += 1