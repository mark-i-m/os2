# OS2 [WIP]

This is a small hobby OS to play around with stuff I have never done before...
it's not intended to be functional, useful, secure, or reliable. It is meant to
be approximately fun to implement.

If you want to see the latest things I am up to, check out the `dev` branch on
this repo. Generally, `master` should compile and run.

# WIP

- Paging
    - `memory::paging::map_region`
    - Need some way of registering valid memory mappings.
    - Page fault handler should check that register and allocate a new page if needed.

- Zero-copy message passing for IPC. To send a message,
    - Remove from sender page tables
    - Remove from sender TLB
    - Insert page into receiver page tables
    - Allow the receiver to fault to map the page. Process receives message via
      the normal future polling.

- I am toying with the idea of not having processes at all, just DAGs of
  continuations which may or may not choose to pass on their capabilities.

# Already implemented

Currently its a little over 1500 LOC (not including comments + whitespace +
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

- Loading a position-independent ELF binary as a user-mode task, running it,
  and exiting via a syscall.

# TODO

Now that I have a mostly functioning basic kernel, I can start playing around
with stuff!

- Need a coherent programming model... how does a user process load other tasks?
- Need to fill out the set of reasonable events.
- Networking? I've never done that before...

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
