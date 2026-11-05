# Are we layout yet?

[arewelayoutyet.mespin.me](https://arewelayoutyet.mespin.me)

A comparison of Rust UI layout libraries:

- [Taffy](https://crates.io/crates/taffy)
- [Morphorm](https://crates.io/crates/morphorm)
- [Torin](https://crates.io/crates/torin)
- [yoga-rs](https://crates.io/crates/yoga), through Rust bindings to the C++ engine
- [ui-layout (WIP 🚧)](https://github.com/098orin/ui_layout-rs), experimental and pinned to a Git revision
- [clay-rs](https://crates.io/crates/clay-layout), through Rust bindings to the C engine
- [Ply](https://crates.io/crates/ply-engine), through its pure-Rust layout context
- [Backer](https://crates.io/crates/backer), an independent dependency-free layout library
- [keru_layout](https://crates.io/crates/keru_layout)

## What is compared?

**Features:** layout models, wrapping, sizing, spacing, constraints, aspect ratios, custom content measurement, and pixel-grid rounding.

**Performance:** daily benchmarks of shared PanGUI workloads, adapted from Taffy's benchmark suite. Results include mean layout times, confidence intervals, and historical runs. Retained-tree engines time uncached layout computation without tree construction. Clay and Ply time their full native passes, including element declaration and render-command generation, because intrinsic sizing occurs during declaration. No commands are consumed or drawn.

Backer runs seven workloads. Its input tree is rebuilt outside timing to clear private resolved state, and its full native resolve/allocation passes are timed without draw callbacks.

The graph offers relative time, total milliseconds, and average nanoseconds per workload node. Per-node values divide the mean and confidence bounds by the expanded template node count, including the root, rather than engine-specific internal nodes. This does not make different tree shapes or layout features equivalent.

The engines have different layout semantics. Missing results (`/`) mean the adapter cannot translate that workload or the library was not included in that historical run. Timings do not establish equivalent output or an overall winner.

Text measurement is not currently compared. See [benchmark provenance](benchmarks/UPSTREAM.md) for attribution and adapter details.
