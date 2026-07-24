# DESIGNING.md — Architecture & Type Design

## 1. Output Format (Mandatory)

Produce exactly this structure. Omit sections that do not apply.

```markdown
## Design Proposal
**Strategy**: [Top-Down | Bottom-Up | Layered | ECS]
**Strategy Selection Criteria**: [Reason for choosing this strategy over alternatives]

### 1. Encapsulation (Level 3)
- Structs: [Nouns representing pure data/state layouts]
- Protection: [Default::default() | Self::new() | Self::try_new()]

### 2. Type Relations (Level 4)
- Has-A: [Struct composition fields; memory layout impact]
- Is-A: [Shared behaviors / trait bounds; minimal bounding justification]

### 3. Component Architecture (Level 5)
- Pattern: [Selected from section 5 Pattern Matrix]
- Justification: [Why this pattern over the next simplest alternative]
- Trade-offs: [What is sacrificed for this choice]

### 4. Artifact
[ASCII Module Hierarchy Tree]
```

## 2. Design Constraints

Apply every constraint below. Do not waive any without a `// user-review:` comment stating the trade-off ceiling and upgrade path.

| Constraint | Enforcement |
|---|---|
| **Fidelity** | Models must completely and accurately represent the domain concept and data flow. No information loss. |
| **Data Transformations over Taxonomy** | Model the problem as a pipeline of data transformations. Structs = pure data; logic = independent functions/systems/isolated methods. |
| **Abstraction** | Hide implementation details. Expose minimal surface area via `pub(crate)`. All items default to private unless explicitly marked by Orchestrator. |
| **No Inheritance** | Composition (struct fields) is the primary structural tool. Shared behavior via traits only. |
| **Data Protection** | Guarantee valid states via constructor validation. No unguarded public mutable fields on domain types. |
| **Redundancy Elimination** | Eliminate duplicate logic across types. Type tree maps directly to the problem domain. |
| **Alignment** | Type relationships must model the data access patterns and problem domain. Names match the user's mental model. |

## 3. Prohibited Patterns (Immediate Validation Failure)

Reject any design containing these patterns. State the violation level and required fix.

1. **Deep trait hierarchies** (simulated inheritance) — Use composition or flat traits instead.
2. **Global mutable state** (`static mut`, unguarded `Mutex`) — Eliminate; use constructor-passed state or channels.
3. **Massive data structs passed as function parameters** — Pass only required fields/primitives/slices.
4. **Public fields on domain types without constructor validation** — Use private fields + `new()`/`try_new()` constructors.
5. **Singleton pattern** (implicit or explicit) — Use `once_cell::sync::Lazy` or `std::sync::OnceLock` with explicit initialization gates.
6. **OOP-style Object Graphs** (deeply nested structs containing behavior and state) — Decouple data from behavior; flatten the graph.

## 4. Strategy Selection Decision Tree

Choose one strategy per design. Apply each criterion in order; stop at the first match.

1. **ECS** — Does the task require processing ≥50 entities with overlapping subsets of properties? → Use ECS with SoA component storage.
2. **Layered** — Do inputs originate from external sources (I/O, network) and flow through distinct validation → transformation → output stages? → Use layered architecture with `pub(crate)` boundaries between layers.
3. **Bottom-Up** — Are there existing reusable primitives in the codebase that compose directly into the required behavior? → Build from proven primitives upward.
4. **Top-Down** — Is this a new subsystem with no compatible existing components and ≥3 distinct domain concepts? → Decompose from system-level contracts downward.

## 5. Idiom Enforcement

Apply these rules without exception.

| Rule | Directive |
|---|---|
| **Polymorphism** | Compile-time monomorphization via generics. `Box<dyn Trait>` only when proven runtime polymorphism is required by the Strategy Selection Decision Tree. |
| **Immutability by default** | `let x` over `let mut x`. Side effects at boundaries only. |
| **Standard method metaphors** | `get_`, `set_` (mut only), `from_`, `into_`, `push_`, `pop_`, `try_new`. Use exclusively; do not invent naming conventions. |
| **Enums as Commands/States** | Model complex states and commands with Rust enums carrying data payloads. Do not use boolean flags or string discriminators for state representation. |

## 6. Pattern Matrix — Rust-Equivalent Mappings

Match the domain requirement to one entry below. Do not invent new structures. When no entry matches, fall back to Section 4's Strategy Selection Decision Tree.

### Creational

| Requirement | Pattern | Rust Implementation |
|---|---|---|
| Simple value types | `Default::default()` or `Self::new()` | — |
| Factory with unknown concrete type at compile time | Abstract Factory | `trait Creator { fn create(&self) -> Box<dyn Product>; }` |
| Family of related objects | Abstract Factory | Trait with multiple related constructor methods |
| 4+ optional parameters or complex initialization | Builder | `struct ConfigBuilder { ... impl ConfigBuilder { fn build(self) -> Config } }` |
| Expensive-to-create types cloned more than recreated | Prototype | `Clone::clone()` |

### Algorithm Abstraction

| Requirement | Pattern | Rust Implementation |
|---|---|---|
| Algorithm chosen at runtime based on context | Strategy | `trait Strategy { fn execute(&self, input: &Input) -> Output; }` |
| Shared skeleton with customizable step logic | Template Method | `fn process<T: FnMut(Item) -> Result<(), E>>(items: &[Item], config: T)` |
| Additive behavior without changing underlying type | Decorator | Iterator adapters (`.map()`, `.filter()`) or wrapper types |

### Coupling / Messaging

| Requirement | Pattern | Rust Implementation |
|---|---|---|
| 99% of cases | Procedure Call | Direct fn call — start here |
| Many-to-many communication with complex interdependencies | Mediator | Central context struct coordinating independent types |
| Multiple handlers where exactly one responds | Chain of Responsibility | `Option<Result<T, E>>` chaining |
| Decoupled notification on state changes | Observer | `crossbeam::channel` or `tokio::broadcast` |
| New operations on stable type hierarchy | Visitor | `trait Visitor<T> { fn visit_foo(&self, foo: &Foo) -> R; fn visit_bar(&self, bar: &Bar) -> R; }` |

### Command Passing

| Requirement | Pattern | Rust Implementation |
|---|---|---|
| Commands with payloads | Native Commands | Enums with payloads (e.g., `Command::Move { x: i32, y: i32 }`) passed via channels |
| Immediate execution needed | Direct Invocation | Direct fn call |
| Defer to subordinate type | Delegate Invocation | Method on owned field |
| Defer, queue, or undoable operations | Command Pattern | `type Action = Box<dyn FnOnce() -> Result<(), E>>` |
| Languages, query systems, config formats | Interpreter | Parser combinators (custom or `nom`) |

### Interface Modification

| Requirement | Pattern | Rust Implementation |
|---|---|---|
| Conform existing type to local contract | Adapter | `impl LocalTrait for ForeignType { ... }` or wrapper type |
| Decouple abstraction from implementation independently | Bridge | Trait + concrete impl structs separated from consumer |
| Simplify complex subsystems behind one interface | Façade | Re-exported convenience API at crate root |
| Expensive creation or shared ownership with mutation | Proxy | `Box<T>` or `Cell`/`RefCell` interior mutability |

## 7. Structural Requirements

Enforce every rule below. Flag violations as immediate failures.

1. Output a module hierarchy map (ASCII tree) for any design touching ≥2 files.
2. Struct names are nouns representing data layouts. Actions belong on isolated methods or decoupled systems.
3. Fields are private by default. Only DTOs and config structs may be public.
4. Constrain generic types with only the bounds the function actually calls. No speculative trait bounds.
