Exoshell uses Semantic Versioning (SemVer) for all machine-readable version numbers.

Version numbers follow:

MAJOR.MINOR.PATCH

Examples:

0.1.0
0.3.3
1.0.0

The numeric version is the authoritative version used by Cargo, Git tags, releases, package managers, automation, and compatibility decisions.

## Release Codenames

Each release also receives a human-readable codename.

Codenames exist for display purposes only and have no semantic meaning. They do not indicate compatibility, stability, feature scope, or release type.

Example display:

Exoshell v0.3.3
codename: packet-goblin

or:

Exoshell 0.3.3 "Packet Goblin"

The codename should be shown in the UI wherever version information is displayed.

## Codename Generation

Release codenames should be generated using a constrained naming scheme.

Recommended format:

<tech-term>-<artifact>

Example tech terms:

kernel
packet
daemon
socket
cache
cipher
terminal
process
satellite
router

Example artifacts:

goblin
wizard
familiar
relic
oracle
ghost
hotdog
cult
seance
rat-king

The lists above are examples only.

Future releases may introduce additional terms as long as they remain consistent with the project's tone.

## Codename Rules

A codename must:

be lowercase
use hyphens as separators
be safe for public display
be memorable
be mildly absurd
be unique across all releases

A codename must not:

contain version numbers
imply compatibility guarantees
contain offensive or discriminatory language
be reused

## Naming Guidance

When creating a release:

Increment the semantic version according to SemVer rules.
Generate a new codename.
Verify that the codename has not been used previously.
Add the codename to release notes and changelog entries.
Preserve existing codenames in project history.


## Historical Codenames
Historical codenames should be tracked in docs/versioning.md below

* 0.1.0 packet-kobold
