# RGB - Game Boy Emulator

A Game Boy emulator written in Rust, based on my original [GameBoyCPP](https://github.com/dimiro1/GameBoyCPP) implementation.

The main goal is to get familiar with Rust and WebAssembly. The frontend will be WebAssembly only.

## Status

This is a work in progress. I have no idea if I'm going to finish it.

## Screenshots

![Tetris running on RGB emulator](imgs/tetris.png)


## AI disclaimer

Not that I have to say this, but: This isn't my first emulator project. I built a similar one in C++ back in university, and I'm generally enthusiastic about gaming emulation. I'm fairly new to Rust and still learning the language's nuances, but I designed the entire system myself. I used Claude Code to speed up the implementation and help me navigate Rust idioms, since it types much faster than I do.

## Running

### CLI
```bash
cargo run -p rgb-cli
```

### WebAssembly
```bash
# Install wasm-pack
cargo install wasm-pack

# Build and run
cd rgb-wasm
wasm-pack build --target web
python3 -m http.server 8000
# Open http://localhost:8000
```
