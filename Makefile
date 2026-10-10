TARGET = riscv32imac-unknown-none-elf
ELF = target/$(TARGET)/release/batmetal
PORT ?=
PORT_ARG = $(if $(PORT),--port $(PORT),)
ESPFLASH ?= espflash
FEATURES ?=
FEATURE_ARGS = $(if $(FEATURES),--features $(FEATURES),)

.PHONY: all build check flash monitor image clean

all: build

build:
	cargo build --release --locked $(FEATURE_ARGS)

check:
	cargo fmt --check
	cargo clippy --locked $(FEATURE_ARGS) -- -D warnings

flash: build
	$(ESPFLASH) flash --chip esp32c6 --flash-size 8mb $(PORT_ARG) --monitor $(ELF)

monitor:
	$(ESPFLASH) monitor $(PORT_ARG) --monitor-baud 115200

image: build
	$(ESPFLASH) save-image --chip esp32c6 --flash-size 8mb --merge $(ELF) batmetal.bin

clean:
	cargo clean
