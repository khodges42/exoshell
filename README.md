# Exoshell

A cognitive shell for engineers who still want the controls.

![DEC inspired logo](docs/images/logo.png)

Exoshell is a local-first cognitive shell designed for practitioners who want AI augmentation without surrendering operational awareness, understanding, or control.

Exoshell is not designed for “vibe coding.”

It is built for:

* engineers who read logs
* people who tune their shell
* operators who want to stay sharp
* terminal users who care about flow-state
* practitioners who believe understanding systems still matters

Exoshell does not try to replace the operator.

It acts more like:

* a cockpit HUD
* a systems copilot
* a cognitive exoskeleton
* an operational overlay for the terminal

The shell remains primary.

The human remains in control.

⸻

Philosophy

Modern AI tooling is rapidly drifting toward opaque automation:

* giant autonomous rewrites
* hidden context accumulation
* blind patch acceptance
* “just trust the agent”
* software generation without understanding

Exoshell takes the opposite position.

We believe:

* skill matters
* operational literacy matters
* flow-state matters
* composability matters
* local-first tooling matters
* understanding systems matters

Exoshell exists to:

* preserve awareness
* reduce cognitive overhead
* accelerate understanding
* enhance practitioner capability
* keep the operator in the loop

Manual Supra, not Waymo.

⸻

Core Principles

Enhance Skill. Do Not Replace It.

Exoshell is intentionally designed for practitioners who want to deepen technical understanding rather than delegate it away.

The system should:

* teach
* explain
* preserve context
* surface uncertainty honestly
* encourage good tooling choices
* help users become more capable over time

The system should not:

* obscure systems
* normalize blind automation
* encourage dependency
* hide execution
* replace understanding with throughput

⸻

The Human Keeps The Controls

Exoshell prefers:

* suggestions over autonomous action
* diffs over blind edits
* transparency over hidden state
* composability over abstraction
* instrumentation over “AI magic”

You should never feel:

“oh god, it’s doing something.”

You should feel:

* informed
* aware
* amplified
* locked in

⸻

Prefer Existing UNIX Tools

If awk is the correct solution, Exoshell should say so.

If jq is cleaner, Exoshell should prefer it.

If the shell already has a deterministic answer, Exoshell should not force AI into the workflow.

The goal is not:

“use AI everywhere.”

The goal is:

“use the right tool while preserving flow.”

⸻

Calm Software

Exoshell should feel like:

* a tuned workstation
* a mech cockpit
* a trusted operator console at 2am

Not:

* a productivity dashboard
* a startup control panel
* a gamified assistant
* an attention machine

The UI should be:

* restrained
* information dense
* aesthetically intentional
* terminal-native
* operationally calm

⸻

What Exoshell Is

Exoshell is:

* shell-adjacent
* terminal-native
* local-first
* practitioner-oriented
* composable
* inspectable
* hackable
* flow-state focused

Exoshell is inspired by:

* GNU / UNIX philosophy
* Emacs
* Vim / Neovim
* Arch Linux
* Gentoo
* tmux
* fish shell
* htop
* lazygit
* cockpit instrumentation
* workstation software

⸻

What Exoshell Is NOT

Exoshell is not:

* an autonomous coding employee
* a hidden surveillance layer
* a cloud lock-in platform
* a manager analytics dashboard
* “AI for everyone”
* a replacement for the shell
* a beginner-first abstraction layer
* a passive screen-watching assistant

Exoshell deliberately makes certain workflows harder:

* blind rewrites
* opaque automation
* no-review patching
* hidden execution
* dependency-forming abstractions
* automation without understanding

⸻

Interaction Model

The core Exoshell loop is:

human explores system
↓
Exoshell observes context
↓
Exoshell enhances awareness
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

Exoshell is designed to feel:

* collaborative
* instrument-like
* operational
* responsive
* trustworthy

Not:

* supervisory
* autonomous
* opaque
* overbearing

⸻

Features (Planned)

Shell-Adjacent Cognitive Overlay

Exoshell lives beside your shell instead of replacing it.

Command Suggestions

Paste, inspect, or execute suggested commands with hotkeys.

Signal Strength

Surface uncertainty honestly instead of hallucinating confidence.

Follow Mode

Ambient repo awareness and contextual operational guidance.

Operational Memory

Searchable markdown-based session notebooks and runbook generation.

Repo Awareness

tree-sitter powered structural understanding.

Stances

Operational behavior modes:

* operator
* audit
* teach
* drift
* quiet
* cockpit

Personalities

Optional expressive overlays:

* minimalist
* mech pilot
* magical girl
* sleepy night operator
* UNIX gremlin

Personality never overrides operational clarity.

⸻

Local-First

Exoshell is designed local-first.

We intentionally optimize for:

* weaker local models
* constrained contexts
* explicit orchestration
* inspectable behavior

If Exoshell works well with local models, stronger hosted models become even more effective.

Cloud backends are optional.

User sovereignty is not optional.

⸻

Technical Direction

Initial stack:

* Rust
* Ratatui
* Crossterm
* Tokio
* tree-sitter
* PTY integration
* markdown notebooks
* local model adapters

⸻

Cultural Direction

Exoshell is practitioner software.

It rewards:

* curiosity
* literacy
* tuning
* craftsmanship
* operational awareness
* intentional workflows

The goal is not to eliminate expertise.

The goal is to amplify it.

⸻

Sacred Rule

Enhance skill. Do not replace it.