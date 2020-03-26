# OS2 [VERY WIP]

This is a small hobby OS to play around with stuff I have never done before...
it's not intended to be functional, useful, secure, or reliable. It is meant to
be approximately fun to implement.

If you want to see the latest things I am up to, check out the `dev` branch on
this repo. Generally, `master` should compile and run.

# WIP

- Userspace
    - Looks like I need to look into the following flags on the GDT entries
      for the kernel code and data segments, which are causing #GP(0x10) on
      iretq after a syscall. 
        - make sure cs desc is readable, ss is writable, both 64-bit (L bit set)

    - Need to load the user code into the new virtual memory region.
        - Using `gz/rust-elfloader` to load ELFs.
        - Hard-code binary into an array with initially... I don't really want
          to implement a file system.

- Execute position-indep binaries in usermode. All executables need to be
  position-independent.

- Paging
    - `memory::paging::map_region`
    - Need some way of registering valid memory mappings.
    - Page fault handler should check that register and allocate a new page if needed.

# Already implemented

Currently its a little over 1300 LOC (not including comments + whitespace +
dependencies). Not bad!

- The kernel itself is continuation-based, rather than using something like
  kthreads. In the first pass, I am just making things work. Later, I might
  go back and make it efficient.

- No timer-based preemption in kernelspace or userspace (though timer
  interrupts do occur so that timers can work). No locks, no multi-threading in
  userspace. Every process is single-threaded and continuation-based. Each
  `Continuation` can return a set of additional continuations to be run in any
  order, an error, or nothing. Continuations can also wait for events, such as
  I/O or another process's termination.

- Single address space. Everything lives in the same address space. Page table
  entry bits are used to disable certain portions of the address space for some
  continuations.

- Small kernel heap for dynamic memory allocation.

- Buddy allocator for physical frame allocation.

- Buddy allocator for virtual address space regions.

- Simple capability system for managing access to resources in the system, such
  as memory regions.

- Switching to usermode and back.

- System calls via `syscall` and `sysret` instructions.

# TODO

- System calls for usermode continuations to indicate their termination and
  schedule the next continuation, possibly pending an event.

- Zero-copy message passing for IPC. To send a message,
    - Remove from sender page tables
    - Remove from sender TLB
    - Insert page into receiver page tables
    - Allow the receiver to fault to map the page. Process receives message via
      the normal future polling.

- I am toying with the idea of not having processes at all, just DAGs of
  continuations which may or may not choose to pass on their capabilities.

# Building

- rust, nightly

  ```txt
  rustc 1.42.0-nightly (0de96d37f 2019-12-19)
  ```

- `llvm-tools-preview` rust distribution component via `rustup component add llvm-tools-preview`

- `cargo xbuild` and `cargo bootimage` via `cargo install cargo-xbuild bootimage`

- `build-essentials` and standard utils: `gcc`, `make`, `ld`, `objcopy`, `dd`

- `qemu` to run

To build and run
```console
$ cd os2/user
$ make
$ cd ../kernel
$ bootimage run # --release for optimized build
```
