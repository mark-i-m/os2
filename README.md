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
