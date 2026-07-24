# IMPLEMENTATION.md — Code & Test TDD Cycle

## 1. Pre-Implementation Ladder (Mandatory)

Stop at the first rung that holds true. Do not skip steps. Do not proceed to write code unless step 7 is reached.

| Step | Directive | Exit Condition |
|---|---|---|
| 1. **YAGNI** | Does this need building? If no, stop. | Requirement cannot be justified against the current operational milestone. |
| 2. **Reuse** | Use existing codebase helpers/utils matching ≥80% of the requirement. | Found ≥1 existing function/struct that solves ≥80% without modification. |
| 3. **Stdlib** | Use stdlib (iterators, `Option`/`Result` chains, `match`) before custom logic. | No existing helper applies but stdlib adapters cover the core algorithm. |
| 4. **Native** | Use native platform features before external solutions. | Stdlib insufficient; OS-level primitives fill the gap. |
| 5. **Dependencies** | Prefer existing `Cargo.toml` crates whose public API directly addresses the requirement. Do not add new crates for transient or one-shot helpers. | No existing crate matches the domain need. |
| 6. **Simplicity** | Can this be expressed as a chain of standard adapters? Make it that short. | A single expression using `filter_map`, `fold`, or equivalent solves the problem. |
| 7. **Minimum Viable Code** | Write the absolute minimum code required to pass the test. | Preceding six steps definitively fail. |

## 2. TDD Cycle (Mandatory for ≥3 decision points)

Execute this cycle without deviation. Do not write implementation code to exercise a test; write code solely to fulfill the contract defined by the initially failing test.

`Requirement → Write Test (fail) → Run (expect fail) → Write Code (pass) → Refactor`

Output this YAML block before writing any code:

```yaml
---
TEST_PLAN:
  FunctionUnderTest: fn_name
  Technique: [UnitTest | IntegrationTest | SystemTest]
  FourSteps:
    Setup: "Preconditions only. No logic."
    Exercise: "Exactly one function call."
    Verify: "One assertion per outcome."
    Teardown: "RAII/scope exit."
  EdgeCases: [empty, zero, boundaries, None]
---
```

## 3. Test Derivation & Execution

### V-Model Mapping

| Test Type | Validates | Scope |
|---|---|---|
| **Unit** | Function/algorithm design | Single module; `#[cfg(test)]` at bottom of source file |
| **Integration** | Cross-module contracts | `tests/` directory; each `.rs` is a separate binary crate |
| **System** | End-to-end headless behavior | Headless execution with CLI flags (e.g., TUI: `--test`) |

### Derivation Matrix — Write tests for all four categories below

| Priority | Category | Directive |
|---|---|---|
| **P0** | Requirements | Validate every stated constraint and acceptance criterion. Non-negotiable. |
| **P1** | Error Conditions | Force logic to handle all I/O failures, invalid parsing, validation breaches without panicking. |
| **P1** | Scenarios | Exercise normal execution flows, standard domain logic paths, expected use cases. |
| **P2** | Boundaries | Test empty input, exact threshold values, overflow limits, `None` vs `Some` distinctions. |

### Test Organization

| Type | Location | Rule |
|---|---|---|
| Unit tests | `#[cfg(test)] mod tests { ... }` at bottom of source file | — |
| Integration tests | `tests/` directory — each `.rs` is a separate binary crate | One test per behavioral scenario |
| Doc tests | `/// \`\`\`rust` blocks for public APIs | — |

### Test Naming Convention

Format: `test_<module>_<scenario>_<expected_result>`

```text
✅ test_validate_schedule_capacity_violation_detected     — describes verified outcome
❌ test_two_tours_with_depot                             — describes implementation, not outcome
```

### When to Skip Tests

Permitted only for these cases:
- Trivial one-liners (identity transformation, no branching).
- Boilerplate mapping input → output without logic.
- Pure pass-through functions (no branching beyond `if let Some`/`if let Ok` for `Option`/`Result` unwrapping).
- Generated code you did not author.

### When Tests Are Mandatory

Required for any function with:
- Conditional logic (`if`/`match`/branches).
- Algorithms (sorting/searching/optimization).
- Validation/parsing logic.
- Boundary conditions.
- Error paths.
- Public API surfaces that other modules depend on.

### Exception — Trivial Code Override

Functions with **exactly one** `if let` branch (solely for `Option`/`Result` unwrapping) and no additional logic fall under the skip clause regardless of public visibility. V-Model testing applies to all functions with **two or more conditional branches**, not fewer.

## 4. Defect Identification

### Assertions

| Assertion | Use When | Build Mode |
|---|---|---|
| `debug_assert!` | Development-only sanity checks for preconditions. Stripped in release builds. | Dev only |
| `assert!` / `assert_eq!` | Production invariants. Failure = broken logic, not bad input. | All modes |

### Trace Variants

| Variant | Implementation | Use When |
|---|---|---|
| Automatic trace | `dbg!()` at key points | Quick inspection during development |
| Flares | `dbg!()` along data path to mark state transitions | Follow a specific variable through multiple functions |
| Memory dump | Serialize intermediate state (e.g., full `Edge` list) | Reproduce state-specific bugs |
| Scoreboards | Aggregate counters tracking items per state | Find where state distribution goes wrong |

## 5. Algorithm Design (Level 1) Constraints

### Efficiency — Memory Layout > Big-O

- Optimize for cache locality and contiguous memory before asymptotic complexity.
- Iterating over flat arrays/slices `O(n)` is faster in Rust than complex pointer-chasing structures `O(log n)`.
- Minimize `Box`, `Arc`, and heap allocations at the hot path.

### Control Flow — Linear Paths Only

Apply these rules without exception:

1. Guard clauses and early returns handle invalid states at the top of functions/loops.
2. Deep nesting is forbidden (maximum two levels of indentation before a guard clause fires).
3. Break complex `&&`/`||` chains into named, independently testable variables.
4. Use `match` with pattern guards over chained `if let`.

```rust
// BAD: buried logic, four conditions in one line
if (vehicle.capacity >= group_size) && (gap_sec >= deadhead_time) && (!is_restricted || gap_sec <= wait_limit) { ... }

// GOOD: named conditions — each self-documenting and independently testable
let has_capacity = vehicle.capacity >= group_size;
let can_deadhead = gap_sec >= deadhead_time;
let satisfies_wait_restriction = !is_restricted || gap_sec <= wait_limit;
if has_capacity && can_deadhead && satisfies_wait_restriction { ... }
```

### Naming — Intent-Revealing Only

- Variable/function names must convey type and intent without comments.
- If a name needs a comment to explain *what* it is, the name is wrong.
- Comments explain WHY (trade-offs, business logic, hardware constraints), not WHAT.

```rust
// BAD: x reveals nothing about type or intent
let x = get_travel_time(&a, &b, &times);

// GOOD: deadhead_duration_sec describes type and intent
let deadhead_duration_sec = get_travel_time(start_node, next_node, &travel_times);
```

### Comments — Only for WHY

```rust
// user-review: O(n²) acceptable for n ≤ 50 nodes. Upgrade to BTreeMap if node count exceeds ~200.
let best_deadhead = find_closest_nonrestricted_node(restricted, &nodes);
```

## 6. Modularization Design (Level 2) Constraints

### Data Transformations over Taxonomy

- Model the problem as a pipeline of data transformations.
- Functions accept flat arrays/slices of data and output transformed states.
- Do not build deeply nested OOP method chains.

### Cohesion — Strong: One Task Per Function

```rust
// BAD: generates, validates, optimizes, assigns — what is this?
fn handle_tour_change(tour: Tour, app: &mut AppState) { ... }

// GOOD: each fn has exactly one responsibility
fn rebuild_edges(app: &mut AppState) -> Vec<Edge> { ... }
fn validate_schedule(edges: &[Edge]) -> HashMap<String, Vec<String>> { ... }
```

### Coupling — Minimal Interface

Pass only required fields. Never pass entire structs for single-field access.

```rust
// BAD: passing whole AppState for one lookup
fn calculate_deadhead(app_state: &AppState) -> i64 { ... }

// GOOD: narrow interface
fn get_travel_time(a: &str, b: &str, times: &HashMap<(String, String), i64>) -> i64 { ... }
```

### Loop Structure — Linear at Top Level

```rust
// BAD: 4 levels deep, work buried inside nested guards
for leg in &legs {
    if let Some(v) = vehicle_map.get(&leg.vehicle_id) {
        if v.capacity >= leg.group_size {
            if check_timeline(leg) && !is_restricted_wait_violated(leg) { /* who can read this? */ }
        }
    }
}

// GOOD: linear, top-level logic with guard clauses
for leg in &legs {
    let vehicle = match vehicle_map.get(&leg.vehicle_id) {
        Some(v) if v.capacity >= leg.group_size => v,
        _ => continue,
    };
    if !check_timeline(leg) { continue; }
    if is_restricted_wait_violated(leg) { continue; }
    // actual work at top level — readable immediately
}
```

### No Copy-Paste

Extract duplicated logic immediately. Every repeated pattern is a maintenance debt and triggers a validation failure.

## 7. Coding Constraints (Hard Rules)

1. No abstractions not explicitly requested. No new dependencies. Delete over add. Touch the fewest files possible.
2. Shortest working diff wins — but only after understanding the problem. Smallest change in the wrong place is a second bug.
3. Mark intentional simplifications with `// user-review:` naming the ceiling and upgrade path.
4. **File Length**: Adhere to limits defined in `REVIEWING.md` section 4 (Ideal: 500 lines, Maximum: 1000 lines). Any file exceeding 1000 lines triggers automatic validation failure.

## 8. Rust Conventions

### Construction Patterns — Always Provide Constructors

| Pattern | Use When |
|---|---|
| `Default::default()` | Simple value types with sensible defaults |
| `Self::new()` | Construction requiring internal parameter assembly or arithmetic validation guaranteed to succeed |
| `Self::try_new()` | Fallible construction (I/O, parsing, external lookups) |

### Trait Bounds Discipline — Minimal Only

```rust
// GOOD: precise bounds
fn find_by_id<T: HasId>(items: &[T], id: &str) -> Option<&T> { ... }

// BAD: over-constrained — demands Debug+Serialize when only Id is needed
fn find_by_id<T: HasId + Debug + Serialize>(items: &[T], id: &str) -> Option<&T> { ... }
```

## 9. Testing Checklist (Mandatory Before Completion)

Verify every item below before declaring completion. Any unchecked item is a validation failure.

- [ ] Non-trivial logic has at least one runnable check
- [ ] Four-step structure followed: Setup → Exercise → Verify → Teardown
- [ ] Edge cases tested: empty input, zero values, `None` vs `Some`, boundaries
- [ ] Error paths exercised
- [ ] Test names describe verified behavior (not implementation)
- [ ] Tests are independently runnable (no ordering dependency)
- [ ] Integration surfaces have at least one cross-module test
