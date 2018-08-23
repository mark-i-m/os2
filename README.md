# OS2 [VERY WIP]

This is a small hobby OS to play around with stuff I have never done before...

Currently its a little over 1000 LOC (including comments + whitespace, not
including dependencies). Not bad!

- No timer-based preemption, no locks, no multi-threading in userspace. Every
  process is single threaded and continuation-based. Each `Continuation` can
  return another continuation, an error, or nothing. Continuations can also
  wait for events, such as I/O or another process's termination.

- Single address space. All executables need to be position-independent. (TODO)

- Zero-copy Message-passing for IPC. (TODO)

- Small kernel heap for dynamic memory allocation.

# Building

- rust, nightly

  ```txt
  rustc 1.30.0-nightly (33b923fd4 2018-08-18)
  ```

- `cargo xbuild` via `cargo install cargo-xbuild`

- build-essentials: gcc, make


To build and run
``` console
$ cd os2
$ make runtext
```

# TODO/WIP

- Buddy allocator for physical frame allocation.

- Kernel reserves a large amount of virtual address space for its own use.

- Zero-copy message passing: to send a message,
    - Remove from sender page tables
    - Remove from sender TLB
    - Insert page into receiver page tables
    - Allow the receiver to fault to map the page. Process receives message via
      the normal future polling.
