export RUSTFLAGS := -Ctarget-cpu=native

EXE = tcheran
SPSA = false
DATAGEN = false

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

ifeq ($(SPSA),true)
	SPSA_FEATURE_ARG := --features spsa
endif

ifeq ($(DATAGEN),true)
	DATAGEN_FEATURE_ARG := --features datagen
endif

default: build

build:
	cargo rustc --manifest-path ./engine/Cargo.toml --bin tcheran --release $(SPSA_FEATURE_ARG) $(DATAGEN_FEATURE_ARG) -- -C target-cpu=native --emit link=$(NAME)
