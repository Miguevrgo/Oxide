ENGINE = oxide
LXENGINE = oxide
EXT :=
VERSION := $(shell grep '^version' Cargo.toml | head -n1 | cut -d '"' -f2)

V3_LINUX = $(LXENGINE)-$(VERSION)-linux-x86_64-v3
V3_WINDOWS = $(LXENGINE)-$(VERSION)-windows-x86_64-v3.exe

all:
	RUSTFLAGS="-C target-cpu=native" cargo build --release

# Publishing a release (Linux + Windows)
publish: $(V3_LINUX) $(V3_WINDOWS)

$(V3_LINUX):
	RUSTFLAGS="-C target-cpu=native" cargo build --release
	cp target/release/$(ENGINE) $(V3_LINUX)

$(V3_WINDOWS):
	RUSTFLAGS="-C target-cpu=native" cargo build --release --target x86_64-pc-windows-gnu
	cp target/x86_64-pc-windows-gnu/release/$(ENGINE).exe $(V3_WINDOWS)

