# Choose tools
CC = gcc

LD = ld

OBJCOPY = objcopy
DD = dd

# Choose options
RUSTOPT = 
ASOPT = -O0 -g

RUSTFLAGS = "-Z no-landing-pads"
ASFLAGS += -m64 ${ASOPT}

# Lists of files
RUSTFILES = $(shell find -name '*.rs')
SFILES = $(wildcard *.S) $(wildcard *.s)

OFILES = $(subst .s,.o,$(subst .S,.o,$(SFILES)))

# General rules
.PHONY: all clean

# by default build, but do not run
all: kernel.img

%.o: %.S
	${CC} ${ASFLAGS} -c -o $@ $<

%.o: %.s
	${CC} ${ASFLAGS} -c -o $@ $<

libkernel.a: ${RUSTFILES} Cargo.toml
	RUSTFLAGS=${RUSTFLAGS} CARGO_INCREMENTAL=1 xargo build --target x86_64-unknown-elf ${RUSTOPT}
	@if [ "${RUSTOPT}" = "--release" ]; then  \
		cp target/x86_64-unknown-elf/release/libkernel.a . ;  \
	else \
		cp target/x86_64-unknown-elf/debug/libkernel.a . ; \
	fi

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

clean-all: clean
	xargo clean
