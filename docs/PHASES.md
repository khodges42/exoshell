# Roadmap

Detailed milestone task lists live in `docs/tasks/`.

The roadmap is organized around a simple principle:

**Enhance skill. Do not replace it.**

Each phase expands Exoshell's capabilities while preserving operator visibility, understanding, and control.

---

# Phase 1: Shell-Aware Chat

Phase 1 establishes the foundation.

Exoshell can communicate with local or remote models, provide shell-aware command suggestions, save transcripts, and operate as a shell-adjacent assistant.

The goal is not automation.

The goal is a reliable operator-facing interface that never executes commands automatically and always keeps the human in control.

---

# Phase 1.5: Context Engine

Phase 1.5 introduces explicit context management.

Context becomes a first-class system with:

* providers
* provenance
* budgeting
* inspection
* pinning
* prioritization
* pruning

The operator can see what context exists, where it came from, how large it is, and whether it will be included in model requests.

This phase establishes the foundation for visible, user-controlled context.

---

# Phase 2: Operational Overlay

Phase 2 transforms Exoshell from a simple chat client into an operational assistant.

This phase adds:

* stances
* context-aware prompt assembly
* command review workflows
* destructive command detection
* command explanation
* context panels
* operator-focused interaction patterns

Exoshell begins to feel like a workstation tool rather than a terminal chatbot.

---

# Phase 3: Daily Driver

Phase 3 turns Exoshell into a useful companion that can remain open throughout an entire work session.

This phase introduces:

* Git-native project awareness
* project-local notebooks
* working sets
* repository summaries
* repository search
* command observation
* command history awareness
* "what happened here?" workflows
* operational memory

The focus is workflow support and situational awareness rather than deep code intelligence.

Exoshell should begin to feel like a trusted operator console.

---

# Phase 3.5: Repository Intelligence

Phase 3.5 introduces lightweight semantic understanding of software projects.

This phase adds:

* symbol discovery
* tree-sitter integration
* architecture notes
* code review workflows
* semantic search
* patch suggestions
* repository knowledge linking

Repository understanding becomes deeper while preserving Exoshell's principles of inspectability and operator control.

Patch suggestions remain suggestions.

Exoshell never modifies repositories automatically.

---

# Phase 4: Context As Cargo

Phase 4 makes context visible.

Rather than treating context as an internal implementation detail, Exoshell presents it as operator-managed cargo.

This phase adds:

* cargo views
* cargo manifests
* prompt preview
* context timelines
* context presets
* signal visualization
* context dashboards

The operator can inspect exactly what information is being carried into a conversation, how much it costs, where it originated, and why it is present.

Trust emerges from visibility rather than hidden automation.

---

# Future Phases

Future milestones may explore:

* richer TUI experiences
* advanced workflow automation
* additional language support
* collaborative workflows
* optional autonomous systems

Any future capability must remain consistent with Exoshell's core principle:

**Enhance skill. Do not replace it.**
