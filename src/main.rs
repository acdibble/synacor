use std::fs;

#[repr(u8)]
#[derive(Debug)]
enum Op {
    Halt,
    Set,
    Push,
    Pop,
    Eq,
    Gt,
    Jmp,
    Jt,
    Jf,
    Add,
    Mult,
    Mod,
    And,
    Or,
    Not,
    Rmem,
    Wmem,
    Call,
    Ret,
    Out,
    In,
    Noop,
}

impl TryFrom<u16> for Op {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Halt),
            1 => Ok(Self::Set),
            2 => Ok(Self::Push),
            3 => Ok(Self::Pop),
            4 => Ok(Self::Eq),
            5 => Ok(Self::Gt),
            6 => Ok(Self::Jmp),
            7 => Ok(Self::Jt),
            8 => Ok(Self::Jf),
            9 => Ok(Self::Add),
            10 => Ok(Self::Mult),
            11 => Ok(Self::Mod),
            12 => Ok(Self::And),
            13 => Ok(Self::Or),
            14 => Ok(Self::Not),
            15 => Ok(Self::Rmem),
            16 => Ok(Self::Wmem),
            17 => Ok(Self::Call),
            18 => Ok(Self::Ret),
            19 => Ok(Self::Out),
            20 => Ok(Self::In),
            21 => Ok(Self::Noop),
            _ => Err(format!("received unknown op code: {value}")),
        }
    }
}

#[derive(Debug)]
enum Arg {
    Literal(u16),
    Register(usize),
}

impl TryFrom<u16> for Arg {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0..=32767 => Ok(Self::Literal(value)),
            32768 => Ok(Self::Register(0)),
            32769 => Ok(Self::Register(1)),
            32770 => Ok(Self::Register(2)),
            32771 => Ok(Self::Register(3)),
            32772 => Ok(Self::Register(4)),
            32773 => Ok(Self::Register(5)),
            32774 => Ok(Self::Register(6)),
            32775 => Ok(Self::Register(7)),
            _ => Err(format!("unable to convert {value} to argument")),
        }
    }
}

#[derive(Debug)]
struct Instruction {
    op: Op,
    a: Option<Arg>,
    b: Option<Arg>,
    c: Option<Arg>,
}

impl Instruction {
    fn new(op: Op) -> Self {
        Self {
            op,
            a: None,
            b: None,
            c: None,
        }
    }
}

const MEMORY_SIZE: usize = 0b0111_1111_1111_1111;

struct VM {
    memory: [u16; MEMORY_SIZE],
    registers: [u16; 8],
    stack: Vec<u16>,
    pc: usize,
}

impl VM {
    fn new() -> Self {
        Self {
            memory: [0; MEMORY_SIZE],
            registers: [0; 8],
            stack: Vec::new(),
            pc: 0,
        }
    }

    fn load(&mut self, bytes: &[u8]) -> Result<(), String> {
        for (slice, dest) in bytes.chunks(2).zip(self.memory.iter_mut()) {
            match slice.get(0..2) {
                Some(&[lo, hi]) => *dest = ((hi as u16) << 8) | (lo as u16),
                _ => return Err("failed to load file".to_string()),
            }
        }

        Ok(())
    }

    fn read_next(&mut self) -> Result<u16, String> {
        let value = *self.memory.get(self.pc).ok_or("failed to get next u16")?;
        self.pc += 1;
        Ok(value)
    }

    fn read_argument(&mut self) -> Result<Arg, String> {
        self.read_next()?.try_into()
    }

    fn read_instruction(&mut self) -> Result<Instruction, String> {
        let op = self.read_next()?.try_into()?;
        let mut inst = Instruction::new(op);

        match inst.op {
            Op::Halt | Op::Noop | Op::Ret => (),
            Op::Out | Op::Jmp | Op::Push | Op::Pop | Op::Call | Op::In => {
                inst.a = Some(self.read_argument()?);
            }
            Op::Jt | Op::Jf | Op::Set | Op::Not | Op::Rmem | Op::Wmem => {
                inst.a = Some(self.read_argument()?);
                inst.b = Some(self.read_argument()?);
            }
            Op::Add | Op::Eq | Op::Gt | Op::And | Op::Or | Op::Mult | Op::Mod => {
                inst.a = Some(self.read_argument()?);
                inst.b = Some(self.read_argument()?);
                inst.c = Some(self.read_argument()?);
            }
        }

        Ok(inst)
    }

    #[inline(always)]
    fn get_value(&self, arg: Arg) -> u16 {
        match arg {
            Arg::Literal(value) => value,
            Arg::Register(reg) => self.registers[reg],
        }
    }

    fn run(&mut self) -> Result<(), String> {
        let mut input = String::new();
        let mut chars = input.chars();

        while self.pc < self.memory.len() {
            let instruction = self.read_instruction()?;
            match instruction {
                Instruction { op: Op::Halt, .. } => break,
                Instruction { op: Op::Noop, .. } => continue,
                Instruction { op: Op::Ret, .. } => {
                    if let Some(value) = self.stack.pop() {
                        self.pc = value as usize
                    } else {
                        break;
                    }
                }
                Instruction {
                    op: Op::Out,
                    a: Some(arg),
                    ..
                } => {
                    let value = self.get_value(arg);
                    if let Ok(ch) = char::try_from(value as u8) {
                        print!("{ch}")
                    } else {
                        return Err(format!("failed to cast {value} to char"));
                    }
                }
                Instruction {
                    op: Op::Jmp,
                    a: Some(arg),
                    ..
                } => self.pc = self.get_value(arg) as usize,
                Instruction {
                    op: Op::Push,
                    a: Some(arg),
                    ..
                } => self.stack.push(self.get_value(arg)),
                Instruction {
                    op: Op::Pop,
                    a: Some(Arg::Register(a)),
                    ..
                } => {
                    if let Some(value) = self.stack.pop() {
                        self.registers[a] = value;
                    } else {
                        return Err(format!("called pop on an empty stack"));
                    }
                }
                Instruction {
                    op: Op::Call,
                    a: Some(a),
                    ..
                } => {
                    self.stack.push(self.pc as u16);
                    self.pc = self.get_value(a) as usize
                }
                Instruction {
                    op: Op::In,
                    a: Some(Arg::Register(reg)),
                    ..
                } => {
                    self.registers[reg] = match chars.next() {
                        Some(ch) => ch,
                        None => {
                            input.clear();
                            std::io::stdin()
                                .read_line(&mut input)
                                .or(Err("failed to read from stdin"))?;
                            chars = input.chars();
                            match chars.next() {
                                Some(ch) => ch,
                                None => return Ok(()),
                            }
                        }
                    } as u16;
                }
                Instruction {
                    op: Op::Jt,
                    a: Some(a),
                    b: Some(b),
                    ..
                } => {
                    if self.get_value(a) != 0 {
                        self.pc = self.get_value(b) as usize
                    }
                }
                Instruction {
                    op: Op::Jf,
                    a: Some(a),
                    b: Some(b),
                    ..
                } => {
                    if self.get_value(a) == 0 {
                        self.pc = self.get_value(b) as usize
                    }
                }
                Instruction {
                    op: Op::Set,
                    a: Some(Arg::Register(reg)),
                    b: Some(b),
                    ..
                } => self.registers[reg] = self.get_value(b),
                Instruction {
                    op: Op::Not,
                    a: Some(Arg::Register(reg)),
                    b: Some(b),
                    ..
                } => self.registers[reg] = !(!0b0111_1111_1111_1111 | self.get_value(b)),
                Instruction {
                    op: Op::Rmem,
                    a: Some(Arg::Register(reg)),
                    b: Some(b),
                    ..
                } => self.registers[reg] = self.memory[self.get_value(b) as usize],
                Instruction {
                    op: Op::Wmem,
                    a: Some(a),
                    b: Some(b),
                    ..
                } => self.memory[self.get_value(a) as usize] = self.get_value(b),
                Instruction {
                    op: Op::Add,
                    a: Some(Arg::Register(a)),
                    b: Some(b),
                    c: Some(c),
                } => self.registers[a] = (self.get_value(b) + self.get_value(c)) % 32768,
                Instruction {
                    op: Op::Eq,
                    a: Some(Arg::Register(a)),
                    b: Some(b),
                    c: Some(c),
                } => {
                    let value_b = self.get_value(b);
                    let value_c = self.get_value(c);
                    self.registers[a] = if value_b == value_c { 1 } else { 0 }
                }
                Instruction {
                    op: Op::Gt,
                    a: Some(Arg::Register(a)),
                    b: Some(b),
                    c: Some(c),
                } => {
                    let value_b = self.get_value(b);
                    let value_c = self.get_value(c);
                    self.registers[a] = if value_b > value_c { 1 } else { 0 }
                }
                Instruction {
                    op: Op::And,
                    a: Some(Arg::Register(a)),
                    b: Some(b),
                    c: Some(c),
                } => self.registers[a] = self.get_value(b) & self.get_value(c),
                Instruction {
                    op: Op::Or,
                    a: Some(Arg::Register(a)),
                    b: Some(b),
                    c: Some(c),
                } => self.registers[a] = self.get_value(b) | self.get_value(c),
                Instruction {
                    op: Op::Mult,
                    a: Some(Arg::Register(a)),
                    b: Some(b),
                    c: Some(c),
                } => {
                    self.registers[a] =
                        (self.get_value(b) as u32 * self.get_value(c) as u32) as u16 % 32768
                }
                Instruction {
                    op: Op::Mod,
                    a: Some(Arg::Register(a)),
                    b: Some(b),
                    c: Some(c),
                } => self.registers[a] = self.get_value(b) % self.get_value(c),
                _ => return Err(format!("unable to handle instruction: {instruction:?}")),
            }
        }

        Ok(())
    }
}

fn main() {
    let mut vm = VM::new();
    let program = fs::read("challenge.bin").expect("failed to read file");
    vm.load(&program).unwrap();
    vm.run().unwrap();
}
