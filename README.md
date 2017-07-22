# OS2

A minimal OS kernel aimed at simplified code and usermode programming experience.

    - no timer-based preemption

    - no demand-based paging: the process needs to explicitly, individually
      request every page it intends to use.

    - no shared memory: all communication is through zero-copy message passing

    - no multithreading: all concurrency is exists as message-passing
      single-threaded processes

    - a quantum of execution is measured in the number of system calls the
      process has in its budget. Each system call uses part of the quantum.
      Think of it this way: a process gets N tokens at the beginning of its quantum,
      which it can spend to do system calls. Each system call costs one token.

    - Assumption: processes may be greedy, but they are actually trying to
      accomplish useful work...

    - Benefits:

        - simplicity: demand paging, shared memory, and interrupts don't need
          to be implemented

        - easier programming model:

            - programmer doesn't need synchronization primitives at all since
              they know they will never be interrupted between system calls

            - (not sure if this is easier or not, but ...) all synchronization
              is done via message passing

            - programmer knows exactly how much memory is being used because
              they have to allocate it all manually!

        - incentivises processes to be efficient

            - less memory usage (since getting memory requires a syscall)

            - less kernel/usermode switching (since these usually happen
              because of syscalls, page faults, interrupts, etc.)

    - Observation: Maybe concurrent programming is hard because it gives you
      too many options!

# TODO

## Global State
- Each core has its own global state object, which is allocated at boot and inaccessable from other cores (no locking necessary)

- There is one global state which is actually global. It is self-synchronizing and we avoid using it if possible

- There is a static which points to each of these objects, which are dynamically allocated at boot time

## Memory management/allocation

### Phys mem allocator

- Buddy heap

- Question: where to keep heap metadata?

- Question: how can we use rust to make sure allocs/frees are sane? Should we use Box or something similar?

- Question: Should we use reference counting and allow objects to be shared around? Or can we get away with requiring that no more than one process has access to an object (normal rust)? (My preference is the latter)

### Slab allocators

- Question: do I want slab allocators? Are they worth it? (maybe try to integrate them later)

### Zero-copy message passing

- Question: how to implement this?

    - I think I want no shared memory (for safety)

    - However, that means that I need to mess with page tables / flush TLB pages every time a message is passed :/

    - Capability-based with exchange heap or something?

    - Another idea: since the send/receiving process has to request access to the memory where the message will go anyway, perhaps we can just do the remapping then... but this really only works for the first time a message is received, right?

## Scheduling

- No preemption

- Single threaded processes only

- Question: Shall I require multiple processors for this OS? (probably no)

- Question: How to set a watchdog over process run time? Use interrupts at low freq? (probably yes)

### Option 1

- Lock-free per-process work queues

- To share work, a processor can put some tasks in a shared area and others can take it

### Option 2 (preferred)

- Biased locking on per-process work queues

- Work stealing schedulers on each processor

- Question: Maybe time to read about Cilq?
