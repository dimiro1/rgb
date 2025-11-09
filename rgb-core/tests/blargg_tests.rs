/// Blargg test ROM suite
///
/// This module contains test runners for various Blargg test ROMs
/// that validate Game Boy emulator accuracy.
use rgb_core::cartridge::Cartridge;
use rgb_core::system::GameBoy;

const SERIAL_DATA: u16 = 0xFF01;
const SERIAL_CONTROL: u16 = 0xFF02;

/// Common test runner for Blargg test ROMs
///
/// Runs a test ROM and collects serial output until the test completes.
/// Returns the collected output string and whether the test passed.
fn run_blargg_test(rom_name: &str, max_instructions: u64) -> (String, bool) {
    let rom_path = format!(
        "{}/{}",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../test-roms"),
        rom_name
    );

    let cartridge =
        Cartridge::load(&rom_path).expect(&format!("Failed to load test ROM: {}", rom_name));
    let mut gameboy = GameBoy::with_cartridge(cartridge);

    let mut output = String::new();

    println!("Running {}...", rom_name);

    for i in 0..max_instructions {
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

        // Progress indicator for longer tests
        if i > 0 && i % 50_000_000 == 0 {
            println!("\n[{} million instructions executed]", i / 1_000_000);
        }

        // Check for completion
        if output.contains("Passed") || output.contains("Failed") {
            println!(
                "\nTest completed after {} million instructions",
                i / 1_000_000
            );
            break;
        }
    }

    let passed = output.contains("Passed");
    (output, passed)
}

/// Helper function to print test results
fn print_test_results(test_name: &str, output: &str, passed: bool) {
    println!("\n=== {} RESULTS ===", test_name.to_uppercase());
    println!("{}", output);
    println!("====================\n");

    if passed {
        println!("✓ {} PASSED!", test_name);
    } else {
        println!("✗ {} FAILED", test_name);

        // Parse and display individual test failures for cpu_instrs
        if test_name.contains("CPU") {
            for i in 1..=11 {
                let test_str = format!("{:02}:ok", i);
                if output.contains(&test_str) {
                    println!("  ✓ Test {:02}: PASSED", i);
                } else {
                    println!("  ✗ Test {:02}: FAILED or incomplete", i);
                }
            }
        }
    }
}

/// Test CPU instructions - This is the main test that validates all CPU instructions work correctly
#[test]
fn test_cpu_instrs() {
    println!("\n=== Blargg CPU Instructions Test ===");
    println!("This tests all CPU instructions for correct behavior.\n");

    let (output, passed) = run_blargg_test("cpu_instrs.gb", 500_000_000);
    print_test_results("CPU Instructions", &output, passed);

    assert!(
        passed,
        "CPU instruction tests failed! See output above for details."
    );
}

/// Test instruction timing - validates the cycle count for each instruction
#[test]
fn test_instr_timing() {
    println!("\n=== Blargg Instruction Timing Test ===");
    println!("This tests the cycle timing of all CPU instructions.\n");

    let (output, passed) = run_blargg_test("instr_timing.gb", 100_000_000);
    print_test_results("Instruction Timing", &output, passed);

    assert!(
        passed,
        "Instruction timing test failed! See output above for details."
    );
}
