# Per-tool implementation source lists for the compiler.
#
# Add dev/src/foo.cpp to the tools that use it by adding `foo` below. For
# subdirectories, use the path without `.cpp`, such as `parser/foo`.

FRONTEND_SOURCE_SET_TARGETS := abimangle pptoken posttoken ppexpr preproc cppgm++ lowiropt lowir lowir2native
FRONTEND_TEST_RUNNER_SOURCE_ID := support/testing/test_runner
# The one source that includes the generated cppgm_builtin_host_config.h
# (the host compiler's macros and include directories, probed at build time).
FRONTEND_BUILTIN_CONFIG_SOURCE_ID := driver/host_environment

# Shared phases: translation phases 1-3 (the pp-token lexer), literal and
# operator classification, phases 5-7 (post-tokens, with identifier
# interning and the arena it uses), controlling-expression
# evaluation and phase 4 (the preprocessor). Tool output adapters are listed
# only by their tools.
FRONTEND_PHASE_BASENAMES_pplex := support/unicode/utf8 preprocess/source/source_buffer preprocess/source/translation_reader preprocess/lexer/char_classes preprocess/lexer/pp_lexer preprocess/lexer/pp_lexer_literals preprocess/lexer/pp_lexer_punctuators
FRONTEND_PHASE_BASENAMES_literals := types/fundamental_type lex/gnu_builtins lex/token_kind lex/ud_suffix lex/number_literal lex/interchange_float lex/text_literal
FRONTEND_PHASE_BASENAMES_posttoken := support/memory/arena lex/identifier_table lex/string_concatenation lex/post_tokenizer
FRONTEND_PHASE_BASENAMES_ppexpr := preprocess/expr/pp_arithmetic preprocess/expr/controlling_expression
# Translation phase 4: directives, macro replacement and the file stack.
FRONTEND_PHASE_BASENAMES_preprocess := preprocess/preprocessor_names preprocess/source/source_files preprocess/source/source_stack preprocess/macro/hide_sets preprocess/macro/macro_definition preprocess/macro/feature_probes preprocess/macro/macro_expander preprocess/preprocessor preprocess/preprocessor_directives

# Translation phases 7-8 as far as PA5 goes: the parser and its tree, with
# the dump view.
FRONTEND_PHASE_BASENAMES_syntax := syntax/parse_token_stream syntax/name_scopes syntax/parser_state syntax/parser syntax/parser_names syntax/parser_declarators syntax/parser_specifiers syntax/parser_declarations syntax/parser_statements syntax/parser_expressions syntax/syntax_unit

# PA6's semantic layer: interned types, scopes and lookup, the declaration
# analyzer and integral constant evaluation; PA7's expression typing,
# conversions, calls and initialization. The --emit-types and
# --emit-semantics dumps (sema/semantic_dump, sema/semantics_dump) are
# linked only by cppgm++.
FRONTEND_PHASE_BASENAMES_sema := sema/type_table sema/lookup sema/integral_arithmetic sema/constant_evaluator sema/constant_interpreter sema/constant_expressions sema/constant_complex sema/constant_construction sema/constant_statements sema/layout sema/primary_bases sema/conversions sema/string_literals sema/analyzer sema/analyzer_types sema/analyzer_declarations sema/analyzer_classes sema/analyzer_bodies sema/analyzer_expressions sema/analyzer_literals sema/analyzer_member_pointers sema/analyzer_calls sema/analyzer_builtins sema/analyzer_asm sema/asm_statements sema/analyzer_bindings sema/pretty_function sema/analyzer_initialization sema/analyzer_construction sema/analyzer_copy sema/analyzer_conversions sema/analyzer_operators sema/analyzer_access sema/analyzer_virtual sema/analyzer_constexpr sema/analyzer_traits sema/analyzer_exceptions sema/analyzer_exception_sets sema/analyzer_lambdas sema/analyzer_range_for sema/analyzer_rtti sema/analyzer_initializer_lists sema/analyzer_handlers sema/function_templates sema/class_templates sema/template_specializations sema/template_packs sema/variable_templates sema/template_names sema/template_addresses sema/template_dependence sema/template_fixed_expressions sema/template_placeholders syntax/ast_clone

# PA8's LowIR model: names, types, literals, the program and its builder,
# and the validator. The text reader and writer (lowir/text_*) are tool
# adapters, listed only by the tools that read or write LowIR text; the
# construction exercises only by lowir.
FRONTEND_PHASE_BASENAMES_lowir := support/memory/arena lowir/name_table lowir/program lowir/builder lowir/literals lowir/validator

# PA9's ABI naming: the canonical ABI graph, the vocabulary tables and the
# one Itanium encoder, which later stages link to name their symbols. The
# fact-file reader (abi/facts/) is the abimangle tool adapter.
FRONTEND_PHASE_BASENAMES_abi := support/memory/arena abi/itanium/abi_graph abi/itanium/abi_encoder

FRONTEND_OBJ_BASENAMES_abimangle := $(FRONTEND_PHASE_BASENAMES_abi) abi/facts/abi_fact_reader abi/facts/abi_fact_definitions
FRONTEND_OBJ_BASENAMES_pptoken := $(FRONTEND_PHASE_BASENAMES_pplex) preprocess/tokens/pp_token_emitter
FRONTEND_OBJ_BASENAMES_posttoken := $(FRONTEND_PHASE_BASENAMES_pplex) $(FRONTEND_PHASE_BASENAMES_literals) $(FRONTEND_PHASE_BASENAMES_posttoken) lex/post_token_writer
FRONTEND_OBJ_BASENAMES_ppexpr := $(FRONTEND_PHASE_BASENAMES_pplex) $(FRONTEND_PHASE_BASENAMES_literals) $(FRONTEND_PHASE_BASENAMES_ppexpr)
FRONTEND_OBJ_BASENAMES_preproc := $(FRONTEND_PHASE_BASENAMES_pplex) $(FRONTEND_PHASE_BASENAMES_literals) $(FRONTEND_PHASE_BASENAMES_posttoken) $(FRONTEND_PHASE_BASENAMES_ppexpr) $(FRONTEND_PHASE_BASENAMES_preprocess) lex/post_token_writer
# PA10's procedural lowering: the typed LowIR program built from the
# semantic facts (symbols through the PA9 encoder); the LowIR text writer is
# the --emit-lowir view.
FRONTEND_PHASE_BASENAMES_lower := lower/program_lowering lower/definition_order lower/symbol_lowering lower/placeholder_mangling lower/local_mangling lower/global_lowering lower/constant_lowering lower/local_static_lowering lower/function_lowering lower/statement_lowering lower/object_lowering lower/copy_lowering lower/conversion_lowering lower/allocation_lowering lower/aggregate_lowering lower/cleanup_lowering lower/expression_lowering lower/discard_lowering lower/intrinsic_lowering lower/soft_float_lowering lower/complex_lowering lower/vector_lowering lower/member_pointer_lowering lower/condition_lowering lower/vtable_lowering lower/covariant_lowering lower/itanium_layout lower/virtual_base_lowering lower/rtti_lowering lower/dispatch_lowering lower/dynamic_type_lowering lower/initializer_list_lowering lower/exception_lowering lower/lexical_cleanup_lowering lower/tracked_lifetime_lowering

FRONTEND_OBJ_BASENAMES_cppgm++ := $(FRONTEND_PHASE_BASENAMES_pplex) $(FRONTEND_PHASE_BASENAMES_literals) $(FRONTEND_PHASE_BASENAMES_posttoken) $(FRONTEND_PHASE_BASENAMES_ppexpr) $(FRONTEND_PHASE_BASENAMES_preprocess) $(FRONTEND_PHASE_BASENAMES_syntax) syntax/ast_render syntax/ast_printer $(FRONTEND_PHASE_BASENAMES_sema) sema/semantic_dump sema/semantics_dump abi/itanium/abi_graph abi/itanium/abi_encoder lowir/name_table lowir/program lowir/builder lowir/literals lowir/validator lowir/text_writer lowir/local_names $(FRONTEND_PHASE_BASENAMES_lower) lex/post_token_writer
# PA32's LowIR optimizer: the passes over the typed program (opt/), linked
# by lowiropt and by cppgm++'s --emit-lowir and object paths.
FRONTEND_PHASE_BASENAMES_opt := opt/function_ir opt/scalar_fold opt/value_simplify opt/dead_code opt/memory_forwarding opt/copy_coalescing opt/constant_propagation opt/cfg_cleanup opt/jump_threading opt/aggregate_slots opt/wide_integers opt/slot_promotion opt/eh_regions opt/loops opt/loop_passes opt/loop_rotation opt/switch_formation opt/select_formation opt/call_graph opt/inliner opt/interprocedural opt/optimizer
FRONTEND_OBJ_BASENAMES_lowiropt := $(FRONTEND_PHASE_BASENAMES_lowir) lowir/text_lexer lowir/text_parser lowir/text_parser_body lowir/text_reader lowir/text_writer lowir/local_names $(FRONTEND_PHASE_BASENAMES_opt)
FRONTEND_OBJ_BASENAMES_lowir := $(FRONTEND_PHASE_BASENAMES_lowir) lowir/text_lexer lowir/text_parser lowir/text_parser_body lowir/text_reader lowir/text_writer lowir/local_names lowir/exercises
# PA24's native backend: selection and allocation (native/select), x86-64
# emission, the runtime routines and the ELF image.
FRONTEND_PHASE_BASENAMES_native := native/select/call_abi native/select/analysis native/select/allocation native/select/global_allocation native/select/register_allocation native/select/loop_split native/select/operands native/select/emit_values native/select/emit_divide native/select/emit_convert native/select/emit_memory native/select/emit_calls native/select/emit_control native/select/host_eh native/select/host_eh_tables native/select/branch_layout native/select/block_placement native/select/frame_layout native/select/shrink_wrap native/select/mir_cleanup native/select/selector native/x86/assembler native/x86/emitter native/x86/emitter_floating native/x86/emitter_memory native/image/executable_file native/image/global_data native/image/image native/runtime/runtime
# lowir2native's whole-program pipeline and its MIR dump view.
FRONTEND_OBJ_BASENAMES_lowir2native := $(FRONTEND_PHASE_BASENAMES_lowir) lowir/text_lexer lowir/text_parser lowir/text_parser_body lowir/text_reader $(FRONTEND_PHASE_BASENAMES_native) native/backend native/mir/mir_writer
# PA25's compile and link modes: the relocatable object model, its ELF
# encoding and the object back end (native/object), the linker and its
# runtime library (native/link), and the driver that runs them.
FRONTEND_PHASE_BASENAMES_object := native/object/object_file native/object/elf_object_writer native/object/elf_object_reader native/object/host_unwind native/object/function_census native/object/object_compiler
FRONTEND_PHASE_BASENAMES_link := native/link/routine_builder native/link/runtime_support native/link/runtime_library native/link/linker
# lowir2native links an executable needing link mode's runtime library.
FRONTEND_OBJ_BASENAMES_lowir2native += $(FRONTEND_PHASE_BASENAMES_object) $(FRONTEND_PHASE_BASENAMES_link)
FRONTEND_OBJ_BASENAMES_cppgm++ += $(FRONTEND_PHASE_BASENAMES_native) $(FRONTEND_PHASE_BASENAMES_object) $(FRONTEND_PHASE_BASENAMES_link) driver/host_environment driver/toolchain
# PA32: the optimizer, and the LowIR reader for LowIR object inputs.
FRONTEND_OBJ_BASENAMES_cppgm++ += lowir/text_lexer lowir/text_parser lowir/text_parser_body lowir/text_reader $(FRONTEND_PHASE_BASENAMES_opt)
