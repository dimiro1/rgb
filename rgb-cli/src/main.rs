use rgb_core::{io, system::State};

fn main() {
    let state = State::new();

    // Print all initial register values
    println!("AF: 0x{:04X}", state.af());
    println!("BC: 0x{:04X}", state.bc());
    println!("DE: 0x{:04X}", state.de());
    println!("HL: 0x{:04X}", state.hl());
    println!("SP: 0x{:04X}", state.sp());
    println!("PC: 0x{:04X}", state.pc());

    // print the P1 memory address
    println!("P1: 0x{:04X}", state.read(io::P1))
}
