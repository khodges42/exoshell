# AGENTS.md

Instructions for AI agents contributing to Exoshell.

## Engineering

* Prefer the simplest solution that satisfies the requirement.
* Do not implement speculative future requirements.
* Extend existing systems before creating new ones.
* Determine whether an existing implementation can be extended before introducing a new subsystem, abstraction, or dependency.
* Always defer to the user when requirements are unclear or tradeoffs are uncertain.
* New dependencies require justification.
* Never duplicate functionality unless doing so avoids unnecessary dependency complexity.

## Documentation

* Use direct, humble technical language, Write like an engineer, not a marketer.
* Be accurate and honest about capabilities and limitations.
* Record known bugs, limitations, and technical debt in `docs/BUGS.md`.
* If there is information worth communicating after completing work, append it to `docs/devlog/YYYY-MM-DD.md`.

## Versioning

* Bump the version for any user-visible change or major feature.
* Follow `docs/versioning.md`.

## Source of Truth
Review before significant changes:

* `docs/DESIGN.md`
* `docs/PHASES.md`
* `docs/tasks/*`
* `docs/versioning.md`

If instructions conflict:

1. AGENTS.md
2. Documentation
3. Code
