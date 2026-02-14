ENGINE = oxide
LXENGINE = oxide
EXT :=
VERSION := $(shell grep '^version' Cargo.toml | head -n1 | cut -d '"' -f2)

EXE ?= $(ENGINE)

all:
	RUSTFLAGS="-C target-cpu=native" cargo build --release
	cp target/release/$(ENGINE) $(EXE)
