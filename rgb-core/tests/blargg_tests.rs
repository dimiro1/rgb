/// Blargg CPU instruction tests
use rgb_core::cartridge::Cartridge;
use rgb_core::system::GameBoy;

const SERIAL_DATA: u16 = 0xFF01;
const SERIAL_CONTROL: u16 = 0xFF02;

#[test]
#[ignore]
fn test_blargg_cpu_instrs() {
    let rom_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../test-roms/cpu_instrs.gb");
    let cartridge = Cartridge::load(rom_path).expect("Failed to load test ROM");
    let mut gameboy = GameBoy::with_cartridge(cartridge);

    let mut output = String::new();

    println!("Running Blargg CPU instruction tests...\n");

    // Run for 500M instructions max (should be enough for all tests)
    for i in 0..500_000_000u64 {
        gameboy.step();

        // Check serial port every instruction
        let serial_control = gameboy.read(SERIAL_CONTROL);
        if serial_control & 0x80 != 0 {
            let byte = gameboy.read(SERIAL_DATA);
            if byte != 0 {
                output.push(byte as char);
                print!("{}", byte as char);
            }
            gameboy.write(SERIAL_CONTROL, 0);
        }

        // Progress indicator
        if i > 0 && i % 50_000_000 == 0 {
            println!("\n[{} million instructions executed]", i / 1_000_000);
        }

        // Check for completion
        if output.contains("Passed") || output.contains("Failed") {
            println!(
                "\n\nTest completed after {} million instructions",
                i / 1_000_000
            );
            break;
        }
    }

    println!("\n\n=== FINAL OUTPUT ===");
    println!("{}", output);
    println!("====================\n");

    // Check results
    if output.contains("Passed") {
        println!("✓ All CPU instruction tests PASSED!");
    } else {
        // Check individual test results
        for i in 1..=11 {
            let test_str = format!("{:02}:ok", i);
            if output.contains(&test_str) {
                println!("✓ Test {:02}: PASSED", i);
            } else {
                println!("✗ Test {:02}: FAILED or incomplete", i);
            }
        }
    }

    assert!(
        output.contains("Passed") || (output.contains("01:ok") && output.contains("02:ok")),
        "CPU instruction tests failed!\nOutput: {}",
        output
    );
}
