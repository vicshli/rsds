# rsds

Some concurrent rust data structures.

This is a (slow but ongoing) effort to port data structures in
[The Art of Multiprocessor Programming](https://www.amazon.com/Art-Multiprocessor-Programming-Revised-Reprint/dp/0123973376)
to Rust. `rsds` follows the book's Java implementations in spirit, but the
implementations could differ significantly because of how Rust works.

## Roadmap

- LinkedLists (ch. 9)
  - [x] `CoarseList` (implemented as `CoarseSet`)
  - [x] `FineGrainedList` (implemented as `FineGrainedSet`)
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
- HashMaps (related to ch. 13 on HashSets)
  - Closed addressing
    - [x] `CoarseHashSet` (implemented as `CoarseMap`)
    - [x] `StripedHashSet` (implemented as `StripedMap`)
    - [ ] `RefinableHashSet`
    - [ ] `LockFreeHashSet` (recursive split-ordering)
  - Open addressing
    - [ ] `PhasedCuckooHashSet`
    - [ ] `StripedCuckooHashSet`
    - [ ] `RefinableCuckooHashSet`
- SkipLists (ch. 14)
  - [ ] `LazySkipList`
  - [ ] `LockFreeSkipList`
