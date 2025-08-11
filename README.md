<div align="center">
  <img src="oxide_logo.png" alt="logo" width="200"/>
</div>

# <div align="center"> Oxide </div>

<div align="center">

  [![License][license-badge]][license-link]
  [![lichess-badge]][lichess-link]
  
</div>

## Overview

`Oxide` started as a simple terminal-based chess project in my personal repository [Projects](https://github.com/Miguevrgo/Projects), but it evolved into something much bigger. A professor proposed it as a challenge, and I took it seriously, transforming it from a basic command-line game into a competitive UCI chess engine. The goal? To learn, to enjoy coding something complex without a GUI, and to build a clean, efficient, and readable piece of software.

This engine is designed to be both a learning experience and a playground for exploring chess programming. It leverages bitboards for board representation, implements move generation with obstruction difference techniques, and aims to balance simplicity with performance. Whether you're a chess enthusiast or a Rust programmer, I hope you find something interesting here!

## Features

- **Complete Chess Implementation:** Fully functional chess rules, including castling, en passant, promotions, and check/checkmate detection.
- **Bitboards:** Efficient board representation using bitboards for fast move generation and evaluation.
- **Obstruction Difference:** Move generation optimized with obstruction difference for sliding pieces.
- **UCI Compliance:** Generally compatible with UCI (Universal Chess Interface), making it playable in tools like CuteChess, Lichess (via bots), or any UCI-supporting GUI.
- **Alpha-Beta Pruning:** Search algorithm with alpha-beta pruning with search enhancements, single-threaded for more comprehensive read.
- **Simplicity & Readability:** Codebase designed to be as straightforward as possible while maintaining decent performance.
- **Inspiration:** Built with insights from the [Chess Programming Wiki](https://www.chessprogramming.org/), top engines like [Carp](https://github.com/dede1751/carp),[Berserk](https://github.com/jhonnold/berserk) and [Akimbo](https://github.com/jw1912/akimbo), which was heavily consulted for the network implementation, The Oxide net provided with the release is trained using [linrock](https://huggingface.co/datasets/linrock/test80-2024/tree/main) datasets and [bullet](https://github.com/jw1912/bullet) trainer.

## Objectives & Planned Improvements

`Oxide` is now a decent engine, however there is still room for improvement:

- **Enhance Search:** Using tables such as continuation history. Testing some prunings for the negamax move loop.
- **NNUE**: I would like testing and learning other configurations.

## Getting Started

### Prerequisites
- Rust (stable, install via [rustup](https://rustup.rs/)).
- (If you want a GUI) An UCI compliant (e.g., [CuteChess](https://cutechess.com/)).

### Build & Run
   ```bash
   git clone https://github.com/Miguevrgo/Oxide.git
   cd Oxide
   RUSTFLAGS="-C target-cpu=native" cargo build --release
   ./target/release/oxide
```
If you want to perform a perft test or benchmark just run:
```bash
  RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture
```
If you prefer a given position test, you can use the UCI interface


[license-badge]:https://img.shields.io/github/license/miguevrgo/Oxide?style=for-the-badge&label=license&color=success
[license-link]:https://github.com/Miguevrgo/Oxide/blob/main/LICENSE
[lichess-link]:https://lichess.org/@/OxideEngine
[lichess-badge]:https://img.shields.io/badge/Play%20Oxide_Engine%20-v1-yellow?logo=lichess&style=for-the-badge
