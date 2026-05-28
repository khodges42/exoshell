Exoshell Design Document (Draft v0.1)

Overview

Exoshell is a local-first cognitive shell for practitioners.

It is designed for engineers, operators, hackers, and technical users who want AI augmentation without surrendering understanding, agency, or operational control.

Exoshell is not an autonomous coding employee.

It is not designed to maximize software generation throughput at the expense of comprehension.

It is not optimized for “vibe coding.”

Exoshell exists to enhance skill, preserve flow-state, increase operational awareness, and keep the human firmly in control of the machine.

The philosophy is simple:

Manual Supra, not Waymo.

The operator remains the pilot.

⸻

Core Principles

1. The Human Keeps The Controls

Exoshell augments operators. It does not automate them away.

The system should prefer:

* suggestion over execution
* visibility over hidden behavior
* review over blind action
* composability over abstraction
* understanding over delegation

Exoshell should never create the feeling:

“oh god, it’s doing something.”

The user should always feel:

* oriented
* informed
* in control
* capable of intervention

⸻

2. Enhance Skill. Do Not Replace It.

Exoshell is intentionally designed for practitioners who want to preserve and deepen technical understanding rather than delegate it away.

The system should:

* teach
* explain
* preserve context
* encourage operational literacy
* surface better tools when appropriate

The system should not:

* encourage dependency
* obscure systems
* normalize blind patch acceptance
* encourage unreviewed giant rewrites

Exoshell is anti-skill-atrophy software.

⸻

3. Prefer Existing UNIX Tools

Exoshell respects the shell ecosystem.

If awk, sed, jq, rg, fd, grep, git, make, or existing UNIX tooling is the correct solution, Exoshell should say so.

Example:

Signal high:
awk is likely the correct tool here.
Suggested:
awk '{print $3}'

Exoshell should never force AI into workflows where deterministic tooling is superior.

⸻

4. Preserve Flow-State

Flow is sacred.

Exoshell exists to preserve:

* momentum
* orientation
* awareness
* context
* operational continuity

The system should feel:

* calm
* responsive
* intentional
* instrument-like

Not:

* noisy
* attention-seeking
* chaotic
* overbearing

⸻

5. Calm Software

Exoshell should feel like:

* a tuned workstation
* a mech cockpit
* a trusted operator console at 2am

Not:

* a startup dashboard
* a productivity app
* a social platform
* a gamified assistant

The UI should be:

* restrained
* high signal
* information dense without clutter
* aesthetically intentional

⸻

6. Visible Systems

Exoshell should expose:

* uncertainty
* reasoning context
* operational state
* diffs
* command execution
* memory usage
* signal quality

Users should never be forced to trust hidden machinery.

⸻

Non-Goals

Exoshell is explicitly NOT:

* a “vibe coding” platform
* a beginner-first abstraction layer
* an autonomous software engineer
* a manager dashboard
* a telemetry surveillance platform
* a cloud lock-in product
* an AI social platform
* an attempt to replace the shell
* a passive screen-watching assistant
* a hidden-memory behavioral profiling system

Exoshell deliberately makes certain workflows harder:

* blind autonomous rewrites
* opaque execution
* no-review patching
* hidden context accumulation
* dependency-forming abstraction
* cloud-dependent workflows
* automation without understanding

⸻

Inspirations

Exoshell is directly inspired by:

GNU / UNIX Philosophy

* composability
* inspectability
* textual interfaces
* user freedom
* small understandable pieces

Emacs

* infinite hackability
* modes
* user ownership
* software as living environment

Vim / Neovim

* speed
* precision
* modal interaction
* mastery through repetition
* config culture

Arch Linux

* explicit control
* user responsibility
* composability
* “build your own machine”

Gentoo

* deep tuning
* obsessive configurability
* understanding through construction
* practitioner culture

tmux / htop / lazygit / fish

* operational calm
* cockpit UX
* high information density
* humane interaction

⸻

User Archetype

The canonical Exoshell user is:

* terminal-native
* technically curious
* workflow-oriented
* skeptical of hidden automation
* interested in improving over time
* willing to tune their environment
* interested in understanding systems rather than merely operating them

Examples:

* SREs
* systems programmers
* security engineers
* infrastructure engineers
* shell enthusiasts
* local-first AI practitioners
* people who think modern AI tooling is moving too fast toward opaque automation

⸻

Interaction Philosophy

Primary Model

Exoshell is shell-adjacent.

The shell remains primary.

Exoshell operates as:

* a cognitive overlay
* operational instrumentation
* contextual augmentation

The operator alternates fluidly between:

* direct shell interaction
* Exoshell prompting
* command review
* contextual follow mode

The system should feel collaborative, not supervisory.

⸻

Core Interaction Loop

human explores system
↓
Exoshell observes context
↓
Exoshell augments awareness
↓
human requests action or insight
↓
Exoshell suggests command or interpretation
↓
human reviews/accepts/modifies
↓
Exoshell interprets results
↓
flow continues

⸻

Command Semantics

By default:

* Exoshell suggests commands
* the user executes them
* execution is explicit
* destructive actions require confirmation

Actions may include:

* paste suggestion
* paste + execute
* explain command
* return stdout/stderr to model
* generate patch
* review diff

⸻

Signal Strength

Exoshell surfaces uncertainty as “signal strength.”

Examples:

* signal: weak
* signal: medium
* signal: high

Low-confidence claims should be clearly surfaced.

The system should prefer:

* clarification
* scoped suggestions
* partial certainty

over hallucinated confidence.

⸻

Stances

Stances define operational behavior.

Examples:

* operator
* audit
* drift
* teach
* quiet
* cockpit

Stances affect:

* verbosity
* interruption frequency
* skepticism
* pacing
* explanation depth
* operational posture

Stances are NOT personalities.

⸻

Personalities

Personalities define aesthetic and expressive behavior.

Examples:

* minimalist
* mech pilot
* magical girl
* sleepy night operator
* UNIX gremlin

Personality affects:

* wording
* cadence
* flavor
* atmosphere

Personality should never compromise:

* operational clarity
* structured output
* shell composability

⸻

Memory Philosophy

Memory is operational, not behavioral.

Exoshell should remember:

* repo structure
* workflows
* troubleshooting knowledge
* active task context
* useful operational discoveries

Memory should NOT silently accumulate:

* invasive behavioral profiling
* emotional manipulation data
* opaque long-term surveillance context

Memory should be:

* inspectable
* scoped
* editable
* searchable
* removable

Memory scopes may include:

* global
* repo
* task
* session
* ephemeral

⸻

Logging Philosophy

Logs exist for:

* transparency
* operational continuity
* searchable notes
* runbook generation
* learning
* debugging

Logs should be:

* human-readable
* text-based
* compact by default
* optionally verbose

The preferred format is markdown-oriented notebook logging.

Exoshell should preserve:

* discoveries
* command outcomes
* architectural observations
* troubleshooting notes

Not raw “session replay.”

⸻

Local-First Philosophy

Exoshell is designed local-first.

Reasons:

* user sovereignty
* reliability
* privacy
* hackability
* reduced lock-in
* stronger runtime design pressure

Cloud models are supported as optional backends, not architectural assumptions.

Designing for weaker local models encourages:

* better orchestration
* better context handling
* better uncertainty semantics
* better UX

⸻

Visual Language

Exoshell should feel like:

* a mech cockpit
* a terminal workstation
* a tactical HUD
* an operator console

The visual language should emphasize:

* restrained palettes
* monochrome foundations
* selective highlights
* tasteful box drawing
* dense but readable layouts
* calm instrumentation

Not:

* dashboard clutter
* excessive animation
* startup aesthetics
* “AI toy” visual language

⸻

Technical Direction (Initial)

Likely stack:

* Rust
* Ratatui
* Crossterm
* Tokio
* tree-sitter
* PTY integration
* markdown-based logging
* local model adapters
* OpenAI-compatible providers

The shell remains external in v1.

Exoshell lives beside the shell rather than replacing it.

⸻

Cultural Direction

Exoshell is practitioner software.

It should encourage:

* workflow craftsmanship
* configuration culture
* literacy
* experimentation
* user ownership
* local-first values
* composability
* sharing tuned environments

The project should resist:

* enshittification
* surveillance incentives
* cloud dependence
* manager-oriented analytics
* skill erosion

Exoshell should feel:

* configurable like Emacs
* precise like Vim
* intentional like Arch
* obsessively tunable like Gentoo

⸻

Sacred Rule

Enhance skill. Do not replace it.

Everything in Exoshell should reinforce this principle.