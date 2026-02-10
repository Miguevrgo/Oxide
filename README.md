<div align="center">
  <img src="oxide_logo.png" alt="Oxide Chess Engine" width="200"/>

  # Oxide

  A UCI chess engine written in Rust.

  [![License][license-badge]][license-link]
  [![lichess-badge]][lichess-link]

</div>

---

## 🔎 About

**Oxide** is a UCI-compliant chess engine built from scratch in Rust, focused on clean code and simplicity. It uses bitboard representation, obstruction difference move generation for sliding pieces, and NNUE evaluation with a network trained on [Leela Chess Zero](https://lczero.org/) data using the [bullet](https://github.com/jw1912/bullet) trainer.

The engine runs a single-threaded alpha-beta search with various enhancements, and is playable on [Lichess](https://lichess.org/@/OxideEngine) or through any UCI-compatible GUI such as [CuteChess](https://cutechess.com/).

## 🏗️ Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)

### Compile

```bash
git clone https://github.com/Miguevrgo/Oxide.git
cd Oxide
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

The binary will be at `./target/release/oxide`.

### Run

Start the engine in UCI mode:

```bash
./target/release/oxide
```

### Bench

Run a fixed-depth search over 50 positions to measure nodes/second:

```bash
./target/release/oxide bench
```

### Perft Test Suite

Run the full perft correctness suite (20 positions with known node counts):

```bash
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture
```

You can also run perft interactively through UCI by typing `perft <depth>` after launching the engine.

## 📦 Releases

Pre-built binaries are available on the [Releases](https://github.com/Miguevrgo/Oxide/releases) page for different CPU targets. If you're unsure which to pick, download the one matching your architecture — or build from source with `target-cpu=native` for best performance on your machine.

## 🙏 Acknowledgements

Oxide draws heavily from the [Chess Programming Wiki](https://www.chessprogramming.org/) and from studying other engines, in particular:

- [Akimbo](https://github.com/jw1912/akimbo) — especially for the NNUE implementation
- [Carp](https://github.com/dede1751/carp)
- [Berserk](https://github.com/jhonnold/berserk)

[license-badge]:https://img.shields.io/github/license/miguevrgo/Oxide?style=for-the-badge&label=license&color=success
[license-link]:https://github.com/Miguevrgo/Oxide/blob/main/LICENSE
[lichess-link]:https://lichess.org/@/OxideEngine
[lichess-badge]:https://img.shields.io/badge/Play%20Oxide_Engine%20-v1-yellow?logo=lichess&style=for-the-badge
