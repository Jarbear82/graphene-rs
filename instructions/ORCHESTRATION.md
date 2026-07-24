# Orchestration — Agentic Pipeline Control

## 1. Role Definitions

Each agent operates within its declared scopes. Violating read/write scope boundaries is an immediate validation failure.

| Agent | Reads | Writes Scope | Constraints |
|---|---|---|---|
| **Researcher** | RESEARCHING.md | Findings documents only; no source changes | Read-only analysis output |
| **Orchestrator** (main agent) | AGENTS.md + ORCHESTRATION.md | All instruction files; delegates work | Must follow routing table and state machine |
| **Designer** | DESIGNING.md | Architecture docs, module trees, pattern proposals | No source code writing permitted |
| **Implementer** | IMPLEMENTATION.md | Code AND test files within declared scope (TDD) | Must satisfy completion signal before reporting success |
| **Reviewer** | REVIEWING.md | Evaluation outputs; no code changes | Must articulate exact refactoring instructions |

## 2. Strict Context Handoff Protocol

When delegating to a sub-agent, output this exact YAML block. No conversational filler. Omit constraint entries that do not apply.

```yaml
---
INVOCATION:
  Role: [Designer|Implementer|Reviewer]
  InstructionFile: instructions/[FILE].md
  WriteScope:
    - src/module/file.rs        # Added/modified files only
    - tests/test_file.rs
  Task: "Declarative description of what to produce"
  Constraints:
    - File length per REVIEWING.md §4 (Ideal: 500; Max: 1000)
    - Follow IMPLEMENTATION.md §[N]
    - Use Result combinators, no unwrap on public I/O
  CompletionSignal: cargo check passes && all tests yield ok
---
```

Sub-agents MUST reply with a `RESULT_SUMMARY` block (see section 3). If the completion signal fails, regress the pipeline to the appropriate predecessor state.

## 3. Sub-Agent Result Format

Every sub-agent returns this exact structure. Omit fields that are N/A.

```yaml
---
RESULT_SUMMARY:
  Done: "One-line description of what was produced"
  FilesChanged:
    - path/to/file.rs — added/modified/deleted: brief rationale
  CompletionSignalStatus: PASS | FAIL
  FailureDetails: "If status is FAIL, explain why"
  RemainingDecisions: []
    - "Question for Orchestrator" (if any)
---
```

## 4. Delegation Logic

### Parallel Delegation — Allowed Only When ALL Apply

- No two agents write the same file or function.
- No data dependency between tasks (verified via caller/callee tracing).
- Each agent has its own `WriteScope` listed explicitly in the YAML invocation.

### Serial Delegation — Required When ANY Condition Applies

- Agent B needs Agent A's output to define scope.
- Tasks touch overlapping files.
- Evaluation gates progress (reviewer must pass before next phase).

### Default Strategy: Parallel with Serial Gates

```
Phase 1: DESIGN     → Serial   (one Designer establishes architecture)
Phase 2: IMPLEMENT+TEST → Parallel  (Implementers work disjoint scopes using TDD)
Phase 3: REVIEW     → Serial   (Reviewer evaluates all results together)
```

## 5. Task Breakdown Criteria

Split any task meeting these conditions into independent sub-tasks:

| Condition | Action |
|---|---|
| Involves more than 2–3 distinct files | Split into one sub-task per file group |
| Spans multiple independent concerns | Split by concern (algorithm, domain types, tests) |
| Contains both read and write operations | Separate the trace from the mutation |

**Example — Correct Breakdown:**
```text
Task A: Extract algorithm module (mod/algorithm/)
Task B: Extract domain types module (mod/domain/)
Task C: Write integration tests for new structure
```

**Example — Incorrect (too broad):**
```text
"Refactor main.rs into modules and add tests"
```

## 6. Orchestrator Decision Flow

Execute every task through this sequence. Do not skip steps or short-circuit the flow based on confidence heuristics.

| Step | Action | Gate |
|---|---|---|
| 1 | Understand problem — trace actual data flow first | Caller/callee map complete |
| 2 | Check YAGNI — does this need building? | Requirement justified against current milestone |
| 3 | Check reuse — does something in the codebase or stdlib already solve it? | Reuse decision recorded; gap analyzed |
| 4 | Route to correct instruction file via routing table (AGENTS.md §3) | All relevant files loaded |
| 5 | Choose parallel vs serial based on write scope overlap | Delegation type justified |
| 6 | Execute delegation with strict YAML handoff block (section 2) | Invocation block outputted |
| 7 | Verify completion signal before closing the task | Signal confirmed PASS or FAIL documented |
| 8 | Check retry count: >3 regressions on same task? | Escalate to human with accumulated failure log |

## 7. Prohibited Orchestration Behaviors

1. Delegating tasks to agents outside their declared WriteScope.
2. Using parallel delegation when any data dependency exists between sub-tasks.
3. Closing a task without verifying the CompletionSignal.
4. Bypassing the REVIEW phase before proceeding to implementation on dependent work.
5. Issuing vague or imperative-free task descriptions (e.g., "make this better").

## 8. Escalation Protocol

When Step 8 triggers (>3 regressions on the same task), execute without exception:

1. **Halt** — Close the current task with `CompletionSignal: FAIL`.
2. **Accumulate** — Collect all `FailureDetails` from every regression cycle.
3. **Escalate** — Output this block to the user:

```yaml
---
ESCALATION:
  Task: "<task description>"
  Regressions: <count>
  FailureLog:
    - Step [N]: <failure reason> → regressed to [phase]
    - ...
  RequiredAction: human-intervention-required
---
```
4. **Do not retry** — The Implementer or Reviewer must never auto-recover after the escalation threshold.

## 9. State Persistence Protocol

Research Reports and intermediate artifacts MUST be filed to disk, not kept in context.

- Research reports → `.pipeline/reports/<module>/research.md`
- Design docs → `.pipeline/designs/<module>/architecture.md`
- Review findings → `.pipeline/reviews/<module>/review.md`

The Orchestrator passes only file *paths* between agents, not the full report content. Agents read their input files directly via the filesystem.

This protocol applies whenever any agent output exceeds 50 lines.
