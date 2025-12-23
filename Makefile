EXE = Tcheran
SPSA = false

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

ifeq ($(SPSA),true)
	SPSA_FEATURE_ARG := --features spsa
endif

openbench:
	cargo rustc --manifest-path ./engine/Cargo.toml --bin engine --release --no-default-features --features release $(SPSA_FEATURE_ARG) -- -C target-cpu=native --emit link=$(NAME)
