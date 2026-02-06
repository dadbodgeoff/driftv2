# Rust Coupling Analysis

## Location
`crates/drift-core/src/coupling/`

## Files
- `analyzer.rs` — Module dependency analysis, cycle detection, hotspot identification
- `types.rs` — `ModuleMetrics`, `DependencyCycle`, `CouplingHotspot`, `UnusedExport`, `CouplingResult`
- `mod.rs` — Module exports

## NAPI Exposure
- `analyze_coupling(files) -> JsCouplingResult`

## TS Counterpart
`packages/core/src/module-coupling/` — Richer analysis with graph building, refactor impact assessment.

## v2 Notes
- Basic coupling analysis works. Needs richer metrics and refactoring suggestions.
