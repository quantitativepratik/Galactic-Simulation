# High-Performance Parallel Simulation Runtime

## Project Overview
This project addresses the **Compiler & Runtime** challenge by implementing a highly optimized, massively parallel N-Body physics engine. 

The goal was to demonstrate how to take a computationally expensive simulation ($O(N^2)$ complexity) and scale it to support "increasingly massive distributed simulations" as requested in the job description, without sacrificing determinism or code readability.

## Key Features
* **Massively Parallel Execution:** Leveraged `rayon` to parallelize the force calculation step, saturating available CPU cores.
* **Determinism Guarantee:** Architected the simulation loop to separate Read (Force Calc) and Write (Integration) phases. This ensures that the parallel execution yields mathematically identical results to the serial execution (verified via unit tests).
* **Observability:** Integrated `tracing` for structured logging, allowing granular performance profiling of individual ticks.
* **Cache Efficiency:** Utilized Structure-of-Arrays (SoA) patterns and contiguous memory layouts to minimize cache misses during the hot loop.

## Performance Benchmarks
Running on a MacBook Pro (M3 Pro), simulating **15,000 bodies**:

| Mode | Execution Time (Avg/Tick) | Speedup |
| :--- | :--- | :--- |
| **Serial (Baseline)** | 325.41 ms | 1x |
| **Parallel (Optimized)** | 55.21 ms | **~5.9x** |

*Note: Parallel overhead prevents scaling at low body counts (<1,000). The system is tuned for high-load scenarios.*

## How to Run

### 1. Run the Simulation
**Parallel Mode (Fast):**
```bash
cargo run --release -- --mode parallel --count 15000
```

**Serial Mode:**
```bash
cargo run --release -- --mode serial --count 15000
```
## Technical Decisions
### Rust & Rayon: Chosen for memory safety and easy access to data parallelism.

### Force Calculation: Implemented a standard brute-force $O(N^2)$ algorithm to simulate a heavy CPU load, making it a suitable candidate for parallel optimization.

### Tracing: Added tracing-subscriber to allow for future integration with observability platforms.