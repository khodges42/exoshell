# Roadmap

Detailed milestone task lists live in `docs/tasks/`.

Current ordering:

* Phase 1 is closed.
* Phase 1.5 establishes the explicit context engine foundation before Phase 2.
* Phase 2 integrates the context engine with stances, safer command handling, and operator controls.
* Phase 3 adds project awareness and operational memory on top of visible, user-controlled context.

# Phase 3 Tasks

Phase 3 goal: provide useful project awareness and operational memory while keeping all context visible, inspectable, and user-controlled.

## P3-001: Project Root Detection

Status: planned.

Outcome: Exoshell can identify and operate against a project root.

Acceptance criteria:

* Detect Git repositories.
* Detect project roots using common markers.
* User can override detected root.
* Current project root is visible.
* Tests cover nested repositories and overrides.

## P3-002: Project Context Model

Status: planned.

Outcome: Exoshell has a structured representation of project information.

Acceptance criteria:

* Project metadata includes root path, repository type, language hints, and discovery timestamps.
* Metadata is serializable.
* Metadata can be displayed in the UI.
* Tests cover serialization and loading.

## P3-003: Repository Summary Generation

Status: planned.

Outcome: Exoshell can generate a high-level summary of a repository.

Acceptance criteria:

* Summary includes major directories.
* Summary includes language breakdown.
* Summary identifies likely entry points.
* Summary avoids reading entire repositories by default.
* Tests cover large repositories.

## P3-004: Repository Ignore Rules

Status: planned.

Outcome: repository scanning remains efficient.

Acceptance criteria:

* Honors .gitignore when practical.
* Supports Exoshell-specific ignore rules.
* Skips common build artifacts.
* Ignore behavior is configurable.
* Tests cover ignore matching.

## P3-005: File Inventory Builder

Status: planned.

Outcome: users can inspect repository contents.

Acceptance criteria:

* Generates file inventories.
* Supports filtering by extension.
* Supports filtering by path.
* Supports size limits.
* Tests cover large inventories.

## P3-006: Symbol Discovery Framework

Status: planned.

Outcome: Exoshell can discover likely symbols without full semantic indexing.

Acceptance criteria:

* Extracts functions, structs, classes, interfaces, and modules where practical.
* Stores symbol metadata.
* Supports lookup by name.
* Supports language-specific extractors.
* Tests cover supported languages.

## P3-007: Language Detector

Status: planned.

Outcome: Exoshell understands repository language composition.

Acceptance criteria:

* Detects primary languages.
* Detects mixed-language repositories.
* Handles generated code separately.
* Displays language summary.
* Tests cover representative repositories.

## P3-008: Git Status Context Provider

Status: planned.

Outcome: users can add current Git state as context.

Acceptance criteria:

* Captures branch.
* Captures modified files.
* Captures staged files.
* Captures untracked files.
* Tests cover detached HEAD and clean repositories.

## P3-009: Recent Commit Context Provider

Status: planned.

Outcome: users can add recent history to context.

Acceptance criteria:

* Supports configurable commit count.
* Includes author, timestamp, message, and changed files.
* Supports filtering by author.
* Supports filtering by path.
* Tests cover repositories with no commits.

## P3-010: Diff Context Provider

Status: planned.

Outcome: users can attach diffs to prompts.

Acceptance criteria:

* Supports staged diffs.
* Supports unstaged diffs.
* Supports specific files.
* Large diffs are truncated visibly.
* Tests cover truncation behavior.

## P3-011: Search Provider Framework

Status: planned.

Outcome: Exoshell can search repositories consistently.

Acceptance criteria:

* Search abstraction exists.
* Supports text search.
* Supports path search.
* Supports symbol search.
* Tests cover provider behavior.

## P3-012: Ripgrep Integration

Status: planned.

Outcome: repository search is fast and useful.

Acceptance criteria:

* Uses ripgrep when available.
* Provides fallback behavior.
* Search results include file and line information.
* Results are attachable as context.
* Tests cover common searches.

## P3-013: Source File Context Commands

Status: planned.

Outcome: users can add code directly to context.

Acceptance criteria:

* Supports adding files.
* Supports adding line ranges.
* Supports adding symbols.
* Supports adding search results.
* Tests cover invalid ranges.

## P3-014: Context Compression Pipeline

Status: planned.

Outcome: large repositories remain usable.

Acceptance criteria:

* Large context can be summarized.
* Compression preserves source references.
* Compression is visible to the user.
* Original context remains inspectable.
* Tests cover compression logic.

## P3-015: Global Notebook Support

Status: planned.

Outcome: users can maintain global notes.

Acceptance criteria:

* Global notebook exists outside repositories.
* Notes are markdown.
* Notes are searchable.
* Notes are user-editable.
* Tests cover creation and loading.

## P3-016: Repository Notebook Support

Status: planned.

Outcome: each repository can maintain its own notebook.

Acceptance criteria:

* Notebook is scoped to repository.
* Notebook persists between sessions.
* Notebook supports markdown.
* Notebook location is configurable.
* Tests cover notebook loading.

## P3-017: Task Notebook Support

Status: planned.

Outcome: users can organize work around tasks.

Acceptance criteria:

* Users can create tasks.
* Tasks have notes.
* Tasks can link context entries.
* Tasks can be completed or archived.
* Tests cover task lifecycle.

## P3-018: Notebook Search

Status: planned.

Outcome: stored discoveries remain useful.

Acceptance criteria:

* Supports keyword search.
* Supports filtering by notebook type.
* Results include source references.
* Results are attachable to prompts.
* Tests cover notebook search.

## P3-019: Discovery Linking

Status: planned.

Outcome: findings become navigable knowledge.

Acceptance criteria:

* Discoveries can link files.
* Discoveries can link symbols.
* Discoveries can link commits.
* Discoveries can link tasks.
* Tests cover link integrity.

## P3-020: Runbook Generation

Status: planned.

Outcome: sessions can produce operational documentation.

Acceptance criteria:

* Generates markdown runbooks.
* Includes commands, findings, and notes.
* Includes timestamps.
* Includes source references.
* Tests cover runbook generation.

## P3-021: Session Summarization

Status: planned.

Outcome: long sessions remain manageable.

Acceptance criteria:

* Session summaries are generated on demand.
* Summaries preserve important discoveries.
* Summaries include linked context.
* Summaries are written to notebooks.
* Tests cover summary generation.

## P3-022: Patch Suggestion Model

Status: planned.

Outcome: Exoshell can suggest code modifications.

Acceptance criteria:

* Suggestions are emitted as diffs.
* Suggestions never modify files automatically.
* Suggestions include rationale.
* Suggestions include uncertainty when appropriate.
* Tests cover patch formatting.

## P3-023: Diff Renderer

Status: planned.

Outcome: code changes are reviewable.

Acceptance criteria:

* Unified diff rendering.
* Syntax-aware formatting where practical.
* Clear additions and removals.
* Works without ANSI color.
* Tests cover rendering.

## P3-024: Patch Export

Status: planned.

Outcome: users can save suggested patches.

Acceptance criteria:

* Exports standard patch files.
* Exports markdown review files.
* Includes metadata.
* Does not modify repository state.
* Tests cover export behavior.

## P3-025: Evidence and Confidence Framework

Status: planned.

Outcome: Exoshell communicates uncertainty consistently.

Acceptance criteria:

* Responses distinguish evidence from inference.
* Confidence labels are documented.
* Confidence metadata can be rendered.
* Confidence survives transcript export.
* Tests cover confidence formatting.

## P3-026: Code Review Stance

Status: planned.

Outcome: Exoshell can behave like a skeptical reviewer.

Acceptance criteria:

* Focuses on correctness.
* Focuses on maintainability.
* Focuses on security.
* Prioritizes findings.
* Snapshot tests cover stance behavior.

## P3-027: Repository Dashboard

Status: planned.

Outcome: users can inspect project state quickly.

Acceptance criteria:

* Shows repository metadata.
* Shows notebook summary.
* Shows recent discoveries.
* Shows current task.
* Displays cleanly in terminal environments.

## P3-028: Tree-Sitter Foundation

Status: planned.

Outcome: semantic parsing foundation exists for supported languages.

Acceptance criteria:

* Tree-sitter integration is optional.
* Supports at least one language initially.
* Symbol extraction can use tree-sitter.
* Fallback path exists when unavailable.
* Tests cover parser loading.

## P3-029: Tree-Sitter Symbol Provider

Status: planned.

Outcome: symbol extraction becomes more accurate.

Acceptance criteria:

* Extracts functions.
* Extracts classes/structs.
* Extracts methods.
* Preserves source locations.
* Tests cover supported languages.

## P3-030: Phase 3 Documentation

Status: planned.

Outcome: users understand project-aware workflows.

Acceptance criteria:

* Documentation covers repositories.
* Documentation covers notebooks.
* Documentation covers patch suggestions.
* Documentation covers confidence reporting.
* Documentation covers limitations.

## P3-031: Phase 3 Test Coverage

Status: planned.

Outcome: repository-awareness features remain reliable.

Acceptance criteria:

* cargo test covers discovery, search, notebooks, patch generation, and confidence handling.
* Snapshot tests protect prompt assembly and patch formatting.
* Manual integration tests remain separate.
* Verification workflow is documented.

## P3-032: Phase 3 Manual Acceptance Test

Outcome: repository awareness milestone is validated.

Acceptance criteria:

* Open a repository.
* Generate repository summary.
* Add git diff context.
* Add recent commit context.
* Search for a symbol.
* Create a task notebook.
* Record discoveries.
* Generate a runbook.
* Request a patch suggestion.
* Export the patch.
* Verify no files are modified automatically.
* Verify all context remains inspectable.
