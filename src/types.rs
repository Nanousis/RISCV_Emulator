use std::io::Write;

pub struct CtrlMessage {
    pub command: Ctrl,
}
pub enum Ctrl {
    RequestFrame,
    Stop,
}

pub struct ScreenMsg{
    pub screen_type: ScreenType,
    pub data: Vec<u8>,
}
pub enum ScreenType {
    TextMode,
    FrameBuffer,
}

pub enum EventType{
    RegWrite{reg: u8, value: u32},
    MemRead{addr: u32, value: u32},
    MemWrite{addr: u32, value: u32},
    FlowChange{new_pc: u32},
    #[allow(dead_code)]
    FlowLink{new_pc: u32, register: u8},
}

pub struct Event{
    pub pc: u32,
    pub opcode: u32,
    pub instr_type: EventType,
}
impl Event{
    pub fn serialize <W: Write>( &self, writer: &mut W ) -> std::io::Result<()> {
        writer.write_all(&self.pc.to_le_bytes())?;
        writer.write_all(&self.opcode.to_le_bytes())?;
        match &self.instr_type {
            EventType::RegWrite{reg, value} => {
                writer.write_all(&[0u8])?;
                writer.write_all(&[*reg])?;
                writer.write_all(&value.to_le_bytes())?;
            }
            EventType::MemRead{addr, value} => {
                writer.write_all(&[1u8])?;
                writer.write_all(&addr.to_le_bytes())?;
                writer.write_all(&value.to_le_bytes())?;
            }
            EventType::MemWrite{addr, value} => {
                writer.write_all(&[2u8])?;
                writer.write_all(&addr.to_le_bytes())?;
                writer.write_all(&value.to_le_bytes())?;
            }
            EventType::FlowChange{new_pc} => {
                writer.write_all(&[3u8])?;
                writer.write_all(&new_pc.to_le_bytes())?;
            }
            EventType::FlowLink{new_pc, register} => {
                writer.write_all(&[4u8])?;
                writer.write_all(&new_pc.to_le_bytes())?;
                writer.write_all(&[*register])?;
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    pub fn serialize_human_readable<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Print the common fields
        write!(writer, "PC=0x{:08X} OPCODE=0x{:08X} ", self.pc, self.opcode)?;

        // Print based on variant
        match &self.instr_type {
            EventType::RegWrite { reg, value } => {
                writeln!(writer, "TYPE=RegWrite REG={} VALUE=0x{:08X}", reg, value)?;
            }
            EventType::MemRead { addr, value } => {
                writeln!(writer, "TYPE=MemRead ADDR=0x{:08X} VALUE=0x{:08X}", addr, value)?;
            }
            EventType::MemWrite { addr, value } => {
                writeln!(writer, "TYPE=MemWrite ADDR=0x{:08X} VALUE=0x{:08X}", addr, value)?;
            }
            EventType::FlowChange { new_pc } => {
                writeln!(writer, "TYPE=FlowChange NEW_PC=0x{:08X}", new_pc)?;
            }
            EventType::FlowLink { new_pc, register } => {
                writeln!(writer, "TYPE=FlowLink NEW_PC=0x{:08X} REG={}", new_pc, register)?;
            }
        }

        Ok(())
    }
}