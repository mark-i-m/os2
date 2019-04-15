# OS2 [VERY WIP]

This is a small hobby OS to play around with stuff I have never done before...

Currently its a little over 1000 LOC (including comments + whitespace, not
including dependencies). Not bad!

- The kernel itself is continuation-based, rather than using something like
  kthreads. (In the first pass, I am just making things work. Later, I might
  go back and make it efficient).

- No timer-based preemption in kernelspace or userspace (though timer
  interrupts do occur so that timers can work), no locks, no multi-threading in
  userspace. Every process is single-threaded and continuation-based. Each
  `Continuation` can return a set of additional continuations to be run in any
  order, an error, or nothing. Continuations can also wait for events, such as
  I/O or another process's termination.

- Small kernel heap for dynamic memory allocation.

- Buddy allocator for physical frame allocation.

# TODO/WIP

- Userspace

- Kernel reserves a large amount of virtual address space for its own use.

- Single address space. All executables need to be position-independent. (TODO)

- Zero-copy message passing for IPC. To send a message,
    - Remove from sender page tables
    - Remove from sender TLB
    - Insert page into receiver page tables
    - Allow the receiver to fault to map the page. Process receives message via
      the normal future polling.

# Building

- rust, nightly

  ```txt
  rustc 1.35.0-nightly (e4c66afba 2019-04-13)
  ```

- `cargo xbuild` via `cargo install cargo-xbuild`

- build-essentials: gcc, make


To build and run
``` console
$ cd os2
$ make runtext
```
