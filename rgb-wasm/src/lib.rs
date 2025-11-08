// WASM bindings for the rgb core library
use rgb_core::{io, mmu::Mmu, system::GameBoy};
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Window};

fn get_output_element() -> Element {
    let window: Window = web_sys::window().expect("no global `window` exists");
    let document: Document = window.document().expect("should have a document on window");
    document
        .get_element_by_id("output")
        .expect("should have an element with id 'output'")
}

fn append_line(text: &str) {
    let output = get_output_element();
    let current = output.inner_html();
    output.set_inner_html(&format!("{}{}<br>", current, text));
}

#[wasm_bindgen]
pub fn run() {
    // Create GameBoy with default Mmu (uses dummy cartridge)
    let gameboy: GameBoy<Mmu> = GameBoy::default();

    // Print all initial register values to the web page
    append_line(&format!("AF: 0x{:04X}", gameboy.af()));
    append_line(&format!("BC: 0x{:04X}", gameboy.bc()));
    append_line(&format!("DE: 0x{:04X}", gameboy.de()));
    append_line(&format!("HL: 0x{:04X}", gameboy.hl()));
    append_line(&format!("SP: 0x{:04X}", gameboy.sp()));
    append_line(&format!("PC: 0x{:04X}", gameboy.pc()));

    // print the P1 memory address
    append_line(&format!("P1: 0x{:04X}", gameboy.read(io::P1)));
}
