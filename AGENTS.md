# SYSTEM DIRECTIVE: AGENTIC PIPELINE

## 1. Understanding Filter

Before executing any request, output this block:

```text
[UNDERSTANDING FILTER]
1. Core Concern: (Design | Implement | Research | Review | Orchestrate)
2. Data Flow/Scope: (List files/functions affected)
3. Root Cause: (Underlying issue, not just symptom)
```

For multi-concern tasks (e.g., design + implement), list all applicable concerns. The filter is mandatory for non-trivial tasks (>1 file or >2 decisions). Skip only for single-line cosmetic changes.

## 2. Pipeline State Machine

Operate strictly within this loop. Do not skip states.

`[ROUTE] -> [RESEARCH] -> [EXECUTE] -> [VERIFY]`

* **ROUTE**: Read AGENTS.md. Identify the target concern via the routing table (section 3). Load the required instruction file. For multi-concern tasks, load all relevant files.
* **RESEARCH**: Trace callers. Map data flow. **DO NOT WRITE CODE YET.**
* **EXECUTE**: Apply the absolute minimum necessary changes. Follow instruction file constraints.
* **VERIFY**: Check all constraints from the loaded instruction file(s). If validation fails, regress to RESEARCH.

## 3. Instruction Routing Table

Load the relevant ruleset for each concern in a task. Do not mix scopes within a single section of work.

| Concern | Load |
| --- | --- |
| Codebase Analysis & Context Mapping | `instructions/RESEARCHING.md` |
| Orchestration & Delegation | `instructions/ORCHESTRATION.md` |
| Architecture, Types & Relations | `instructions/DESIGNING.md` |
| Implementation & TDD | `instructions/IMPLEMENTATION.md` |
| Quality Metrics & Limits | `instructions/REVIEWING.md` |

## 4. Universal Constraints (Absolute Directives)

* **Idiomatic Rust > Data Oriented Design**: Default to idiomatic Rust for simple domains; default to Struct of Arrays and flat layouts when processing ≥100 homogenous entities or when cache locality is measurable.
* **YAGNI**: Build only what is requested. Do not invent or add features, abstractions, or boilerplate not explicitly required by the prompt.
* **Reuse**: If it exists in the codebase or standard library, use it. Never re-implement stdlib.
* **Composition > Inheritance**: Use Rust traits for shared behavior and struct fields (`Has-A`) for data. Never simulate inheritance hierarchies.
* **Delete over Add**: Always prefer removing code to adding it. Touch the fewest files possible.
