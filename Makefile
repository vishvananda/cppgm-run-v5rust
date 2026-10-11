# Exact Rust clone: the assignment Makefiles remain the owning course harness.
EXPERIMENTAL_PAS ?= pa34 pa35
CHECK = python3 /home/vishvananda/work/v5rust-control/clone_check.py
export CARGO_HOME := $(abspath ../cache/cargo)
export CARGO_TARGET_DIR := $(CURDIR)/obj/rust
export CLIPPY_CONF_DIR := $(CURDIR)/dev
export CARGO_BUILD_WARNINGS := deny
.PHONY: all build lint audit-rust test test-harness inception test-debuginfo
all: build
build:
	cargo build --manifest-path dev/Cargo.toml --workspace --release --locked
lint:
	python3 /home/vishvananda/work/v5rust-control/clone_quality.py
audit-rust:
	/home/vishvananda/work/v5rust-control/venv/bin/python /home/vishvananda/work/v5rust-control/rust_audit.py --root $(CURDIR)
test: test-report-through-pa33
test-pa%:
	$(CHECK) stage pa$*
course-pa%:
	$(CHECK) course pa$*
test-report-through-pa%:
	$(CHECK) through pa$*
test-debuginfo:
	$(CHECK) release pa33
test-harness:
	/home/vishvananda/work/v5rust-control/venv/bin/python -m unittest discover -s /home/vishvananda/work/v5rust-control -p 'test_*.py'
inception: build
	python3 /home/vishvananda/work/v5rust-control/clone_bootstrap.py --final
