# REVIEWING.md — Quality Evaluation

## 1. Review Workflow (Mandatory)

Execute every review through this sequence. Do not skip steps or combine steps.

| Step | Action | Gate |
|---|---|---|
| 1 | Identify the concern: algorithm logic, modularization, type design, module relations, or system architecture? Pull up the relevant checklist section below. | Concern classified to L1–L5 level |
| 2 | Assess against metrics: Score each metric by naming its specific canonical level. Do not use bare adjectives. Evaluate strictly against Rust/DOD physical realities. | Every applicable metric scored with canonical level + justification |
| 3 | Verify constraints: Reject files exceeding 1000 lines. Flag untested non-trivial logic immediately. | File length checked; test coverage verified |
| 4 | Enforce the smallest fix: Propose the least invasive structural change addressing the root cause. Never refactor for its own sake. | Proposed fix addresses root cause, not symptoms |

## 2. Quality Metrics (Canonical Levels)

Score each metric using only the levels listed below. Always state both metric name and level with justification.

### Level 1 & 2: Execution & Modularity

| Metric | Canonical Levels | Evaluation Basis |
|---|---|---|
| **Efficiency** | `O(1)` → `O(log n)` → `O(n)` → `O(n log n)` → `O(n²)` → `O(2ⁿ)` | Prioritize memory layout, cache locality, contiguous arrays, minimal pointer chasing. An O(n) flat-iteration outranks an O(log n) heap-allocated tree on modern hardware. |
| **Maintainability — Understandability** | `Obvious` → `Straightforward` → `Deducible` → `Misleading` → `Puzzling` | Respect ownership rules; minimize explicit lifetimes; names reveal intent without comments. |
| **Maintainability — Malleability** | `Configurable` → `Data-driven` → `Adjustable` → `Refactorable` → `Prohibitive` | **Malleability measures how easily *existing* code can be modified.** Borrow-checker compliant refactoring that does not trigger cascading lifetime errors scores higher. |
| **Cohesion** | `Strong` → `Extraneous` → `Partial` → `Weak` | Data transformations are tightly grouped; one responsibility per function. |
| **Coupling** | `Trivial` → `Encapsulated` → `Simple` → `Complex` → `Document` → `Interactive` → `Superfluous` | Narrow interfaces with minimal parameter passing score higher than broad struct-passing. |

### Level 3, 4 & 5: Encapsulation & System

| Metric | Canonical Levels | Evaluation Basis |
|---|---|---|
| **Fidelity** | `Complete` → `Extraneous` → `Partial` → `Poor` | Structs map accurately to physical data flow with no information loss. |
| **Robustness** | `Proven` → `Resilient` → `Strong` → `Tested` → `Fragile` | Graceful error handling via `Result`/`Option`; invalid states made unrepresentable; no `.unwrap()` on public I/O. |
| **Convenience** | `Seamless` → `Easy` → `Straightforward` → `Convoluted` → `Prohibitive` | Standard method metaphors (`get_`, `set_`, `from_`, `into_`, `push_`, `pop_`) lower cognitive burden. |
| **Abstraction** | `Complete` → `Opaque` → `Porous` → `Critical` | Zero-cost abstractions hide memory layout details; minimal `pub` surface. |
| **Adaptability** | `Enabling` → `Straightforward` → `Convoluted` → `Prohibitive` → `Closed` | **Adaptability measures how easily *new* functionality can be added without modifying existing files.** Composition and ECS enable extension; deep inheritance scores lower. |
| **Alignment** | `Complete` → `Extraneous` → `Partial` → `Poor` | Type relationships match the user's mental model and data access patterns. |
| **Redundancy** | `Distinct` → `Minor` → `Critical` → `Redundant` | DRY enforced at compile time via generics and composition, not copy-paste. |

## 3. Strict Evaluation Checklists

Apply each checklist to the relevant concern level. Every unchecked item is a flag requiring action.

### L1 — Algorithm Design

- [ ] Names are intent-revealing (no `x`, no unexplained abbreviations).
- [ ] Comments explain WHY, not WHAT.
- [ ] Paths are linear — guard clauses and early returns, no deep nesting.
- [ ] Boolean chains broken into named variables.

### L2 — Modularization Design

- [ ] Each function has exactly one responsibility (pipeline of data transformations).
- [ ] Zero code duplication — all repeated patterns extracted.
- [ ] Inputs minimized — only required data passed (favor flat arrays/slices, never whole structs for single fields).

### L3 — Encapsulation Design

- [ ] Struct names are nouns representing pure data layouts (not active objects).
- [ ] Fields are private by default; public only for DTOs and config.
- [ ] Invariants validated in constructors (`new` / `try_new`).
- [ ] Constructors always provided (`Default::default()`, `Self::new()`, or `Self::try_new()`).

### L4 — Module/Type Relation Design

- [ ] Composition favored over inheritance (struct fields over trait hierarchies).
- [ ] Trait bounds are minimal — only what the function actually calls.
- [ ] Invariants enforced by the type system, not caller discipline.

### L5 — Component & System Design

- [ ] Components are swappable behind traits.
- [ ] Interfaces are narrow (`pub(crate)` visibility, minimal `pub` surface).
- [ ] Backward compatibility considered — additive changes over breaking ones.
- [ ] Logic decoupled from state (verbs operate on nouns, rather than nouns owning verbs).

## 4. File Length Limits

Enforce these limits without exception. Flag violations as immediate failures.

| Category | Limit | Action on Violation |
|---|---|---|
| **Ideal** | 500 lines | — (target, not gate) |
| **Maximum** | 1000 lines | Split into multiple files within a sub-directory with `mod.rs` exposing the public API. |
| **Exception** | Documented reason as `// user-review:` comment at file top | Only for small, cohesive concerns that legitimately need more space. |

## 5. Required Output Format (Mandatory)

Format every review exactly as shown below. Deviation is a validation failure.

```markdown
## Review: [Filename or Module]

### 1. Algorithm & Modularity
* **Efficiency**: `O(n)` — [Justification based on complexity/memory layout]
* **Maintainability**: `Obvious` / `Adjustable` — [Justification based on ownership/borrow checker]
* **Cohesion**: `Strong` — [Justification]
* **Coupling**: `Encapsulated` — [Justification]

### 2. Encapsulation & System
* **Fidelity**: `Complete` — [Justification]
* **Robustness**: `Strong` — [Justification]
* **Abstraction**: `Opaque` — [Justification]
* **Adaptability**: `Enabling` — [Justification]

### 3. File Constraints
* **Length**: `Pass` (245/1000 lines) OR `FAIL` (1500/1000 — split required)

### ACTION REQUIRED:
[ ] NONE (Pass)
[X] REFACTOR: [Specific, imperative instruction to Implementer]
```

If `ACTION REQUIRED` is not `NONE`, regress the pipeline to the Implementer immediately with the refactoring instructions.
