# OS2

- No timer-based preemption, no locks, no multi-threading (in userspace). Every
  process is single threaded.

- Single address space. All executables need to be position-independent.

- Zero-copy Message-passing for IPC.

- All tasks are Future-based. Every routine can return a `Future`, which is
  pushed to the end of that processor's scheduler list. The cores work through
  their lists in a work-stealing fashion.

# Building

- rust, nightly

  ```txt
  rustc 1.27.0-nightly (2f2a11dfc 2018-05-16)
  ```

- xargo

- build-essentials: gcc, make


To build and run
``` console
$ cd os2
$ make runtext
```

# TODO

## Global State
- Each core has its own global state object, which is allocated at boot and
  inaccessable from other cores (no locking necessary)

- There is one global state which is actually global. It is self-synchronizing
  and we avoid using it if possible

- There is a static which points to each of these objects, which are
  dynamically allocated at boot time

## Memory management/allocation

### Phys mem allocator

- Buddy allocator for physical frames

- Question: where to keep metadata?
    - Need to make sure that allocation doesn't trigger allocation!
    - Would be nice if we didn't have to keep metadata for every page!

### Virtual memory

- Reserve a large portion of the address space for the kernel. Upper half (2^63
  bytes)?

- Kernel heap allocator lives in part of this region of the single address
  space and is backed by the page frame allocator.

- Kernel exposes this heap as `Box`.

### Zero-copy message passing

- To send a message,
    - Remove from sender page tables
    - Remove from sender TLB
    - Insert page into receiver page tables
    - Allow the receiver to fault to map the page. Process receives message via
      the normal future polling.

## Scheduling

- No preemption

- Single threaded tasks only

- Question: How to deal with scheduling events (e.g. receive a message, receive
  hardware interrupt)?
    - Events of various kinds (if they are intended for a user process) go into
      a queue somewhere in the kernel (task struct?). When the future for that
      event is polled, if the event is found, the future returns it.

- Each processor core has a local work queue, and we use a work-stealing
  scheduler to process futures/continuations.
