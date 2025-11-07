EXE = Tcheran

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

openbench:
	cargo rustc --manifest-path ./engine/Cargo.toml --bin engine --release --no-default-features --features release --features spsa -- -C target-cpu=native --emit link=$(NAME)
