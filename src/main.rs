mod context;

use context::SystemState;

fn main() {
    let ctx = SystemState::new();

    // Print all initial register values
    println!("AF: 0x{:04X}", ctx.af());
    println!("BC: 0x{:04X}", ctx.bc());
    println!("DE: 0x{:04X}", ctx.de());
    println!("HL: 0x{:04X}", ctx.hl());
    println!("SP: 0x{:04X}", ctx.sp);
    println!("PC: 0x{:04X}", ctx.pc);
}
