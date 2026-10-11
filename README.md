# cppgm-rust

An exact, safe Rust port of the CPPGM C++ compiler implementation, driven by
the v5rust assignment loop. This is the initial scaffold; the compiler tools
return status 86 until implemented.

Read [CLONE_CONTRACT.md](CLONE_CONTRACT.md), [AGENTS.md](AGENTS.md),
[the conversion plan](docs/rust-clone-plan.md) and [ROADMAP.md](ROADMAP.md).
The C++ oracle is pinned to commit 905bd7e4412a2d77d88694fe6abc7f3e1c27bce1.
The original handouts, tests, regressions, architecture specification and
attribution are retained. See [NOTICE](NOTICE).

Use Rust naming conventions throughout the implementation: snake_case modules
and functions, UpperCamelCase types, SCREAMING_SNAKE_CASE constants, kebab-case
packages. Preserve externally observable C++ spellings and CLI tool names.

`make build`, `make lint`, `make audit-rust`, `make test-pa1`.
The installed protected gate configuration and C++ reference paths are recorded
in `scripts/clone/settings.json`. Reprovision those paths on another machine;
the repository contains no oracle binary or compiler fallback.

PA1-PA33 establish exact behavior. PA34 compiles the original C++ sources and
compares each object and all rebuilt tools with the C++ self-build fixed point.
PA35 is reference material, not an optimization objective for this port.
