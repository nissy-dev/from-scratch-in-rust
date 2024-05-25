use num_derive::FromPrimitive;

#[derive(Debug, PartialEq, FromPrimitive)]
pub enum Opcode {
    End = 0x0B,
    LocalGet = 0x20,
    I32Add = 0x6A,
    Call = 0x10,
}
