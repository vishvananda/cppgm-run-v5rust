# Owner PA29 dependency repair

Frozen C++ advanced from `c2b0d41791ad702b70f01f346b36a390b35b3077` to `b43173ec8d98b27eecf02bccabf973a4083a389f`, matching assignments `22e677f46ffa27a8e003c0b53331c9188d151071`.
PR #83 passed CI; local full report 7084/7084, debug info, self-host through PA5,
architecture/placement/file audits and PA35 smoke pass. PA35 hardware instruction
ratios on identical frozen-workload outputs are .999989 at O0 and 1.000009 at O3.

The approved protected host capture includes Cargo configuration, build.rs,
safe build-owned probe logic and one data-import module. Scope is empty-input
configuration only; macros/includes/query bytes are immutable at runtime. GNU,
strict, include-failure and unavailable-host fallbacks match the generator.
Captured g++ command spelling matches the oracle. Cargo rerun dependencies and
matching checker build inputs avoid owner/worker configuration drift.

The Rust driver answers direct queries and rejects mixed queries in the normal
parser without consuming option arguments as queries. Integration tests cover
queries, -E and -c with no host PATH. Vector lane lowering uses the converted
operand's retained type and evaluates the conversion once; the former crash
case now produces identical objects. The owning PA29 runtime regression comes
from extended and executes through protected course/differential gates.

No unsafe/FFI, compiler fallback, arbitrary generated-code import, acceptance
waiver, manual Ralph state/goal edit, fixture deletion or reference imitation.
All worker audit fixes through 9ec7cde6 are retained. The previous blocked goal
is history; a normal fresh-thread launch resumes the audit after validation.
Raw validation and owner backups: `/home/vishvananda/work/maintenance/rust-pa29-unblock-20261025`. Full PA29 acceptance still belongs to
the worker's final audit and Ralph's phase checks.

Final owner validation: harness, complete Rust quality, PA1–PA28 differential
and course gates, PA29 differential/course checks and identity preservation all
pass. Protected evidence index: /home/vishvananda/work/maintenance/rust-pa29-unblock-20261025/evidence-index.json.
