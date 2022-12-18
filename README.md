# rsds

Some concurrent rust data structures.

This is a (slow but ongoing) effort to port data structures in
[The Art of Multiprocessor Programming](https://www.amazon.com/Art-Multiprocessor-Programming-Revised-Reprint/dp/0123973376)
to Rust.

## Roadmap

- LinkedLists (ch. 9)
  - [x] `CoarseList`
  - [x] `FineGrainedList`
  - [ ] `OptimisticList`
  - [ ] `LazyList`
  - [ ] `LockFreeList`
- Queues (ch. 10)
  - [ ] `BoundedQueue` (a bounded, partial queue)
  - [ ] `UnboundedQueue` (an unbounded, total queue)
  - [ ] `LockFreeQueue` (a lock-free, unbounded queue)
  - [ ] `SynchronousDualQueue` (a dual data structure)
- Stacks (ch. 11)
  - [ ] `LockFreeStack`
  - [ ] `EliminationBackoffStack`
- HashSets (ch. 13)
  - Closed addressing
    - [ ] `CoarseHashSet`
    - [ ] `StripedHashSet`
    - [ ] `RefinableHashSet`
    - [ ] `LockFreeHashSet` (recursive split-ordering)
  - Open addressing
    - [ ] `PhasedCuckooHashSet`
    - [ ] `StripedCuckooHashSet`
    - [ ] `RefinableCuckooHashSet`
- SkipLists (ch. 14)
  - [ ] `LazySkipList`
  - [ ] `LockFreeSkipList`
