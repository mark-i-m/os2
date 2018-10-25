# Choose tools
CC = gcc

LD = ld

OBJCOPY = objcopy
DD = dd

# Output and Input config
TARGET_JSON = x86_64-unknown-elf.json

# Choose options
RUSTOPT =
ASOPT = -O0 -g

ASFLAGS += -m64 ${ASOPT}

# Build up cargo flags
CARGO_TARGET_FLAGS = --target $(TARGET_JSON)
CARGO_OUTPUT_FLAGS = --out-dir `pwd`
CARGO_FLAGS = -Z unstable-options $(CARGO_OUTPUT_FLAGS) $(RUSTOPT) $(CARGO_TARGET_FLAGS)

# Lists of files
RUSTFILES = $(shell find -name '*.rs')
SFILES = $(wildcard *.S) $(wildcard *.s)

# Compiled Objects

OFILES = $(subst .s,.o,$(subst .S,.o,$(SFILES)))

# General rules
.PHONY: all clean

# by default build, but do not run
all: kernel.img

%.o: %.S
	${CC} ${ASFLAGS} -c -o $@ $<

%.o: %.s
	${CC} ${ASFLAGS} -c -o $@ $<

libkernel.a: ${RUSTFILES} Cargo.toml Makefile
	cargo xbuild  ${CARGO_FLAGS}

%.bin: %
	${OBJCOPY} -O binary $< $@

%.img: %.bin
	${DD} if=$< of=$@ bs=512 conv=sync

# kernel build
kernel: ${OFILES} libkernel.a
	${LD} --gc-sections -m elf_x86_64 -N -n -e start --section-start mbr=0x7c00 -Ttext=0x9000 -o kernel ${OFILES} --start-group libkernel.a --end-group

clean:
	rm -f *.o
	rm -f *.a
	rm -f *.img
	rm -f *.bin
	rm -f kernel
	cargo clean
