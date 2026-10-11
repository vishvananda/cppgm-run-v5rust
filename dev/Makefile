TOOLS = pptoken posttoken ppexpr preproc cppgm++ abimangle lowir lowiropt lowir2native
.PHONY: all $(TOOLS)
all $(TOOLS):
	CARGO_HOME=$(abspath ../../cache/cargo) CARGO_TARGET_DIR=$(abspath ../obj/rust) cargo build --manifest-path Cargo.toml --workspace --release --locked
