use rgb_core::{io, mmu::Mmu, system::GameBoy};

fn main() {
    // Create GameBoy with default Mmu (uses dummy cartridge)
    let gameboy: GameBoy<Mmu> = GameBoy::default();

    // Print all initial register values
    println!("AF: 0x{:04X}", gameboy.af());
    println!("BC: 0x{:04X}", gameboy.bc());
    println!("DE: 0x{:04X}", gameboy.de());
    println!("HL: 0x{:04X}", gameboy.hl());
    println!("SP: 0x{:04X}", gameboy.sp());
    println!("PC: 0x{:04X}", gameboy.pc());

    // print the P1 memory address
    println!("P1: 0x{:04X}", gameboy.read(io::P1))
}
