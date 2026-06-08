# si-fibration-timing

> **Proof of Concept:** Fiber bundles model timing channels in concurrent agent systems — the connection IS the conservation law.

## The Insight

In differential geometry, a **fiber bundle** (E, B, π, F) is a space that locally looks like a product B × F but may have non-trivial global structure. The **connection** tells you how to "parallel transport" a fiber element from one base point to another.

For agent timing:
- **Base space B** = task state space (what the agent is doing)
- **Fiber F** = timing manifold (latency, jitter, throughput)
- **Total space E** = all (task, timing) pairs
- **Connection** = conservation law (how timing is preserved across task switches)

The key result: **holonomy** (the gap after transporting around a closed loop) measures how much timing structure is NOT conserved. Zero holonomy = flat connection = perfect conservation.

## What This Proves

1. **Timing conservation is a connection** — parallel transport with conservation=1.0 preserves all timing
2. **Holonomy = conservation violation** — non-zero holonomy means the timing budget leaks
3. **Curvature = drift rate** — sections with high curvature have fast timing degradation
4. **Flat sections are optimal** — constant timing across tasks = zero holonomy

## Usage

```rust
use si_fibration_timing::{Connection, Section, TaskPoint, TimingFiber, FiberBundle};

// Create a connection (0.9 conservation = 90% timing preserved per transport)
let conn = Connection::new(0.9, 0.1);

// Build a section: assign timing to tasks
let mut section = Section::new();
section.add(
    TaskPoint { task_id: 0, state: vec![0.0, 0.0] },
    TimingFiber { latency: 1.0, jitter: 0.1, throughput: 100.0 },
);
section.add(
    TaskPoint { task_id: 1, state: vec![1.0, 0.0] },
    TimingFiber { latency: 1.5, jitter: 0.2, throughput: 80.0 },
);

// Measure curvature (timing drift)
let curvature = section.curvature(&conn);

// Measure holonomy (conservation gap around loops)
let holonomy = section.holonomy(&conn);
```

## Connection to Conservation Law

The conservation law γ + η = C appears here as:
- **γ (durable)** = the connection's conservation parameter
- **η (ephemeral)** = the decay from drift
- **C (total)** = the original timing budget

When γ = 1.0 (perfect conservation), holonomy = 0 and timing is perfectly preserved across all task switches. When γ < 1.0, timing leaks through holonomy.

## Modules

- `Connection` — parallel transport rule with conservation parameter
- `Section` — assignment of timing fibers to task base points
- `FiberBundle` — complete bundle with holonomy group computation
- `curvature()` — measures timing drift rate
- `holonomy()` — measures conservation violation

## License

MIT
