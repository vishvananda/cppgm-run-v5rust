# Current clone baseline — PA29 repair

Reference `b43173ec8d98b27eecf02bccabf973a4083a389f`; matching assignments `22e677f46ffa27a8e003c0b53331c9188d151071`.
Extended PR #83 passed the complete CI matrix and was fast-forwarded to main.
The oracle was rebuilt locally from that exact commit. This update includes all
intervening course changes; all previous anchors and candidate regressions remain.
Host capture uses reviewed protected Rust build inputs and data-only imports.
Ralph examples commit: 0beabba. Prior baseline history remains in Git.

## PA32 statistics adapter — 2026-10-26

Owner installed Ralph `f098bca`: recognize `pa32_opt_stats` under the existing
runtime-measurement rule. Deterministic counters, diagnostics and output files
remain exact; receipts retain raw statistics. Oracle self-comparison passes at
O0–O3; installed harness, preflight, prior-through and quality checks pass.
Evidence: `/home/vishvananda/work/maintenance/v5rust-pa32-stats-adapter-20261026`. The frozen oracle and fixtures are unchanged.
