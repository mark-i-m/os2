

CFLAGS=-std=c99 -m64 -ffreestanding -nostdlib -nodefaultlibs -Wall -Werror -fpie
LDFLAGS=-N -m elf_x86_64 -e start

BINS=test

all: $(BINS)

%.o: %.S
	gcc $(CFLAGS) -c $<

%.o: %.c
	gcc $(CFLAGS) -c $<

test: test.o crt0.o
	ld $(LDFLAGS) -o $@ $^

clean: 
	rm -f *.o $(BINS)
