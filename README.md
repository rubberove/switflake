# Switflake

A swift, wait-free unique ID generator with thread-aware sequencing in Rust.

## Features
- 64-bit IDs with timestamp, node ID, and thread-aware sequence.
- No heap allocations (stack-only).
- Wait-free operation with thread-local sequencing.
- Supports 4,096 nodes and up to 256 IDs per thread per millisecond (max 8 simultaneous threads).

## Bit Layout
- 41 bits: Timestamp (69.7 years).
- 12 bits: Node ID (4,096 nodes).
- 11 bits: Sequence (2,048 IDs per millisecond, including 3-bit thread ID and 8-bit counter).

## TODOS
- [ ] Create well-formatted documentation
- [ ] Performance comparison
- [ ] Conduct real-world testing
- [ ] Dogfooding 