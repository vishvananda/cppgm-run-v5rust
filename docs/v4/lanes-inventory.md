# Course and regression lanes in the source tree

One row per test unit (`x.t`, `x.cpp`, `x.lowir`, `x.program`) with its lane and bucket;
Phase 4 moves each into `paN/tests/<bucket>/` under an audited cluster.

| lane | pa | bucket | units |
|---|---|---|---|
| course | pa1 | `(flat)` | 23 |
| course | pa2 | `(flat)` | 10 |
| course | pa3 | `(flat)` | 12 |
| course | pa4 | `(flat)` | 36 |
| course | pa5 | `(flat)` | 7 |
| course | pa6 | `(flat)` | 16 |
| course | pa7 | `(flat)` | 18 |
| course | pa8 | `(flat)` | 26 |
| course | pa9 | `(flat)` | 14 |
| course | pa10 | `(flat)` | 9 |
| course | pa11 | `(flat)` | 10 |
| course | pa12 | `(flat)` | 18 |
| course | pa13 | `(flat)` | 30 |
| course | pa14 | `(flat)` | 6 |
| course | pa15 | `(flat)` | 11 |
| course | pa15 | `controls` | 5 |
| course | pa16 | `(flat)` | 59 |
| course | pa16 | `controls` | 4 |
| course | pa17 | `(flat)` | 19 |
| course | pa17 | `controls` | 10 |
| course | pa18 | `(flat)` | 8 |
| course | pa19 | `(flat)` | 25 |
| course | pa20 | `(flat)` | 11 |
| course | pa21 | `(flat)` | 23 |
| course | pa22 | `(flat)` | 28 |
| course | pa23 | `(flat)` | 15 |
| course | pa25 | `(flat)` | 22 |
| course | pa26 | `(flat)` | 8 |
| course | pa27 | `(flat)` | 1 |
| course | pa28 | `(flat)` | 3 |
| course | pa29 | `behavior` | 78 |
| course | pa29 | `controls` | 19 |
| course | pa29 | `strict` | 4 |
| course | pa29 | `structural` | 25 |
| course | pa30 | `(flat)` | 12 |
| course | pa31 | `(flat)` | 13 |
| course | pa32 | `(flat)` | 16 |
| course | pa32 | `controls` | 3 |
| course | pa33 | `(flat)` | 3 |
| course | pa34 | `compile` | 7 |
| course | pa34 | `run` | 1 |
| course | pa35 | `compile` | 50 |
| course | pa36 | `link` | 1 |
| course | pa37 | `debuginfo/driver/o3` | 1 |
| course | pa37 | `debuginfo/o1` | 1 |
| course | pa37 | `debuginfo/o3` | 1 |
| course | pa37 | `driver/o1` | 11 |
| course | pa37 | `driver/o3` | 1 |
| course | pa37 | `o0` | 1 |
| course | pa37 | `o1` | 52 |
| course | pa37 | `o2` | 23 |
| course | pa37 | `o3` | 9 |
| course | pa37 | `object-roundtrip` | 17 |
| course | pa38 | `behavior/o1` | 5 |
| course | pa38 | `behavior/o2` | 8 |
| course | pa38 | `behavior/o3` | 1 |
| course | pa38 | `debuginfo/o1` | 2 |
| course | pa38 | `debuginfo/o2` | 2 |
| course | pa38 | `debuginfo/o3` | 1 |
| course | pa38 | `driver` | 2 |
| course | pa38 | `o1` | 11 |
| course | pa38 | `o2` | 5 |
| course | pa38 | `o3` | 2 |
| regression | pa29 | `strict` | 40 |
| regression | pa29 | `structural` | 85 |
| regression | pa37 | `controls` | 27 |
| regression | pa37 | `driver/o1` | 14 |
| regression | pa37 | `driver/o2` | 6 |
| regression | pa37 | `driver/o3` | 1 |
| regression | pa37 | `o0` | 5 |
| regression | pa37 | `o1` | 123 |
| regression | pa37 | `o2` | 36 |
| regression | pa37 | `o3` | 9 |
| regression | pa38 | `behavior/o1` | 5 |
| regression | pa38 | `behavior/o2` | 8 |
| regression | pa38 | `behavior/o3` | 1 |
| regression | pa38 | `controls` | 20 |
| regression | pa38 | `o1` | 25 |
| regression | pa38 | `o2` | 22 |
| regression | pa38 | `o3` | 2 |

## Units

### course/pa1/(flat)

- `100-extra-comments`
- `100-line-splice-comment`
- `100-more-floating`
- `100-pp-number-exponent-sign-boundary`
- `100-raw-string-literal`
- `150-user-defined-literals`
- `150-user-defined-raw-string-literal`
- `200-alternative-include-header-context`
- `200-escaped-universal-name-prefix-string`
- `200-header-name`
- `200-identifier-unicode-e2-ranges`
- `200-non-raw-string-prefix`
- `200-raw-string-delimiter`
- `200-string-escape-sequences`
- `200-trigraphs-and-comments`
- `300-invalid-universal-character-value-bad`
- `300-raw-string-delimiter-bad`
- `300-raw-string-delimiter-trigraph-bad`
- `300-string-hex-escape-bad`
- `300-string-octal-escape-bad`
- `300-trigraph-line-splice-comment-include`
- `300-utf8-bom`
- `400-angle-colon-template-tokenization`

### course/pa2/(flat)

- `200-exponent-like-integer-ud-suffix`
- `200-string-numeric-escape-code-units`
- `200-unicode-character-literals`
- `200-unicode-user-defined-literal-suffix`
- `300-incomplete-floating-exponent-bad`
- `300-invalid-character-string-boundary`
- `300-invalid-floating-literal-shapes`
- `300-multiple-decimal-points-bad`
- `300-string-numeric-escape-out-of-range`
- `300-user-defined-literal-range`

### course/pa3/(flat)

- `100-unicode-character-literals`
- `120-defined-keyword-operand`
- `120-defined-malformed-operands`
- `200-adjacent-subtraction`
- `200-chained-multiplicative`
- `200-chained-shifts`
- `200-operator-precedence`
- `200-signed-unsigned-comparison`
- `300-incomplete-expression-bad`
- `300-logical-line-error-isolation`
- `300-unconsumed-expression-tokens-bad`
- `500-integer-overflow`

### course/pa4/(flat)

- `100-undef-simple`
- `150-hash-outside`
- `200-function-macro-invocation-boundaries`
- `200-invalid-token-macro-replacement`
- `200-whitespaces`
- `250-badvaargs-param`
- `250-badvaargs-undef`
- `250-empty-function-macro-argument`
- `250-goodvaargs`
- `250-nested-macro-argument-commas`
- `250-stringized-argument-not-expanded`
- `300-define-missing-macro-name`
- `300-directive-tokens-from-macro-argument`
- `300-double-hash`
- `300-gnu-variadic-comma-paste`
- `300-invalid-token-paste`
- `300-macro-redefinition-whitespace`
- `300-nonstringized-invalid-macro-argument`
- `300-object-function-macro-redefinition`
- `300-redef`
- `300-token-paste-multiple-parameter-tokens`
- `300-unterminated-function-macro-after-comma`
- `300-unterminated-function-macro-parameter`
- `300-unterminated-function-macro-parameters`
- `300-variadic-only-parameter-must-close`
- `300-variadic-tail-parameter-must-close`
- `400-fun-macro-define`
- `410-trigraph-in-raw-string`
- `500-hello-world`
- `500-raw-string-token-paste`
- `600-hash-from-macro`
- `600-macro-rescan-boundaries`
- `600-parameter-selected-macro-rescans`
- `600-pasted-helper-macro-rescans`
- `600-tail-helper-macro-rescans`
- `600-unavailable-paint-through-function-replacement`

### course/pa5/(flat)

- `200-conditional-exclusion`
- `200-defined-identifier-like-operator`
- `300-defined-operator-misuse`
- `300-line-new-line`
- `400-multiple-source-files`
- `600-predefined-macro-argument-expansion`
- `600-repeated-argument-expansion`

### course/pa6/(flat)

- `150-bare-label-statement`
- `200-new-parenthesized-type-id`
- `200-operator-delete-id`
- `200-operator-function-ids`
- `250-decl-specifiers-without-type`
- `250-member-function-trailing-semicolon`
- `250-member-uniform-initializer`
- `250-named-bitfield-expression`
- `300-empty-case-expression-bad`
- `300-invalid-token-balanced-scan-bad`
- `300-try-without-handler-bad`
- `400-ellipsis-without-comma`
- `500-deep-template-argument-failure-bad`
- `500-nested-template-delimiter-context`
- `500-operator-template-angle-boundary`
- `500-template-name-angle-commit-bad`

### course/pa7/(flat)

- `200-using-directive-anchor-nested`
- `200-using-directive-anchor-sibling`
- `200-using-directive-transitive-extension`
- `220-namespace-name-lookup-shadowing`
- `270-using-declaration-reuse`
- `280-inline-namespace-alias-lookup`
- `280-inline-namespace-qualified-lookup`
- `280-inline-namespace-reopen-bad`
- `320-array-bound-literal-forms`
- `320-array-completion`
- `320-parenthesized-array-abstract`
- `350-parenthesized-parameter-declarators`
- `360-function-void-typedef`
- `370-reference-collapse-alias-chain`
- `500-lookup-cache-invalidation`
- `600-deep-function-type`
- `600-deep-parenthesized-declarator`
- `600-deep-using-directive-chain`

### course/pa8/(flat)

- `120-constant-conversion-linkage`
- `120-constexpr-pointer-cross-tu`
- `120-constexpr-qualified-pointer`
- `120-constexpr-reference-static-object`
- `150-static-thread-local-cross-tu`
- `150-thread-local-redeclaration-agreement`
- `200-char-before-function-alignment`
- `200-function-typedef-declarations`
- `300-constexpr-missing-initializer-bad`
- `300-constexpr-nonconstant-initializer-bad`
- `300-constexpr-zero-id-pointer-bad`
- `300-cross-tu-constant-visibility-bad`
- `300-cv-through-typedef-constant`
- `300-function-typedef-definition-bad`
- `300-invalid-fundamental-specifiers-bad`
- `300-scalar-array-initializer-bad`
- `300-thread-local-redeclaration-mismatch-bad`
- `320-using-directive-ambiguity-bad`
- `350-integer-to-pointer-bad`
- `350-multilevel-qualification-conversion-bad`
- `350-string-to-mutable-pointer-bad`
- `450-cv-dropping-reference-bad`
- `450-lvalue-to-rvalue-reference-bad`
- `500-lookup-cache-invalidation`
- `600-deep-function-type`
- `600-deep-namespace-initialization`

### course/pa9/(flat)

- `100-empty-program`
- `100-empty-program.ref`
- `300-negative-memory-literal-bad`
- `300-unparenthesized-label-offset-bad`
- `300-unparenthesized-negative-immediate-bad`
- `300-unsupported-conversion-opcode-bad`
- `400-negated-wchar-sign-extension`
- `400-negated-wchar-sign-extension.ref`
- `450-raw-literal-width-and-nan`
- `450-raw-literal-width-and-nan.ref`
- `500-long-double-label-alignment`
- `500-long-double-label-alignment.ref`
- `500-string-literal-element-alignment`
- `500-string-literal-element-alignment.ref`

### course/pa10/(flat)

- `dependent-logical-template-argument`
- `function-type-template-argument-leading-named-parameter`
- `known-function-name-with-type-marker`
- `late-nested-class-type-in-inline-member`
- `parameter-shadows-template-comparison`
- `parameter-shadows-template-strict-comparison`
- `template-template-parameter-name-reuse`
- `typedef-shadows-outer-value-in-member-function`
- `value-and-call-condition`

### course/pa11/(flat)

- `200-qualified-scoped-enum-definition-lookup`
- `300-ambiguous-using-directive-type-bad`
- `320-void-variable-bad`
- `321-uninitialized-reference-bad`
- `322-nonenclosing-qualified-definition-bad`
- `323-namespace-alias-reopened-bad`
- `324-declaration-forms-valid`
- `325-complete-class-member-type`
- `namespace-nonstatic-anonymous-union-bad`
- `scoped-enum-integral-comparison-bad`

### course/pa12/(flat)

- `300-deferred-demand-closure`
- `300-floating-bitwise-compound-assignment-bad`
- `300-function-variable-conflict-bad`
- `300-member-cv-overload-identity`
- `300-qualified-lookup-owner-bad`
- `300-static-pointer-integral-cast-bad`
- `300-switch-floating-condition-declaration-bad`
- `300-variable-function-conflict-bad`
- `310-raw-string-underscore-delimiter`
- `320-builtin-abort-arity-bad`
- `conditional-const-enum-preserves-type`
- `distinct-anonymous-type-identities`
- `namespace-static-anonymous-union-injected-members`
- `private-nested-out-of-class-constructor-definition`
- `private-nested-out-of-class-member-definition`
- `qualified-base-data-member-hides-function`
- `qualified-member-scoped-enum-definition`
- `reference-parameter-to-value-call`

### course/pa13/(flat)

- `300-bad-entry-role-declaration`
- `300-bad-global-tls-wrapper-metadata`
- `300-bad-negative-debug-line`
- `300-bad-writable-storage-metadata`
- `400-bad-phi-missing-predecessor`
- `400-bad-phi-placement`
- `400-bad-phi-type`
- `400-phi-control-flow`
- `410-bad-unknown-instruction`
- `420-bad-unknown-arity-metadata`
- `430-bad-redundant-effects-default`
- `431-bad-redundant-unwind-default`
- `432-bad-redundant-return-default`
- `433-bad-redundant-fixed-arity`
- `434-bad-redundant-direct-pass`
- `435-bad-redundant-maycapture`
- `436-bad-redundant-readwrite-access`
- `437-bad-redundant-cpp-linkage`
- `438-bad-false-keep-alias-flag`
- `439-bad-false-prefer-local-flag`
- `440-bad-false-object-root-flag`
- `441-bad-false-force-inline-flag`
- `442-bad-false-inline-hint-flag`
- `443-bad-false-no-inline-flag`
- `444-bad-unknown-symbol-role`
- `450-bad-unknown-symbol-metadata`
- `500-bad-unreachable-as-role`
- `500-terminate-role-metadata`
- `510-inline-hint-metadata`
- `511-inline-hint-global-bad`

### course/pa14/(flat)

- `700-conflicting-ref-qualifiers-bad`
- `700-global-structured-operator`
- `700-inline-operator-parameter-count`
- `700-nonfunction-records-bad`
- `700-structured-thunk`
- `700-typeinfo-name`

### course/pa15/(flat)

- `500-inline-function-lowir-hint`
- `510-no-optimize-predefine-at-o0`
- `520-readonly-const-scalar-storage`
- `canonical-comparison-result`
- `extern-const-definition-retains-external-linkage`
- `generated-temporary-name-collision`
- `in-class-defaulted-assignment-is-weak`
- `integral-literal-floating-initializer`
- `scalar-literal-reference-temporary-lowering`
- `scalar-new-value-initialization-lowering`
- `static-member-callee-declaration`

### course/pa15/controls

- `530-pointer-difference-strength-reduction`
- `540-readonly-scalar-storage`
- `541-volatile-access-markers`
- `542-builtin-memory-alias-boundaries`
- `543-ordinary-pointer-decay`

### course/pa16/(flat)

- `100-aggregate-character-array-udl-bad`
- `100-aggregate-union-active-member`
- `100-alignas-bit-field-bad`
- `100-base-initializer-hidden-by-static-member-bad`
- `100-bit-field-assignment-result`
- `100-bit-field-narrow-storage`
- `100-bit-field-over-width-layout`
- `100-bit-field-repeated-subobject-init`
- `100-builtin-memory-void-pointer-boundary`
- `100-call-conversion-deleted-best-bad`
- `100-call-conversion-nonconst-reference-bad`
- `100-const-scalar-member-default-construction-bad`
- `100-copy-list-explicit-best-constructor-bad`
- `100-defaulted-constructor-throwing-dmi-metadata`
- `100-deleted-constructor-selected-bad`
- `100-destructor-explicit-return-subobjects`
- `100-duplicate-class-definition-bad`
- `100-duplicate-data-member-bad`
- `100-empty-base-nested-zero-offset-collision`
- `100-extern-thread-local-declaration`
- `100-function-noexcept-redeclaration-mismatch-bad`
- `100-implicit-integer-narrowing-fact`
- `100-inherited-constructor-same-signature`
- `100-inherited-private-constructor-derived-friend-bad`
- `100-inline-constructor-copy-init`
- `100-invalid-in-class-static-initializer-bad`
- `100-local-aggregate-array-omitted-members`
- `100-namespace-using-function-conflict-bad`
- `100-nested-class-array-lifecycle`
- `100-nested-constructor-aggregate-reference`
- `100-private-base-destructor-bad`
- `100-private-base-inherited-static-call-bad`
- `100-private-base-inherited-type-bad`
- `100-private-base-inherited-type-member`
- `100-private-defaulted-destructor-bad`
- `100-protected-member-base-object-bad`
- `100-protected-using-base-object-bad`
- `100-qualified-friend-must-be-declared-bad`
- `100-reference-member-default-construction-bad`
- `100-static-thread-local-internal-linkage`
- `100-thread-local-member-not-layout-field`
- `100-union-destructor-does-not-destroy-variants`
- `100-unused-deleted-subobject`
- `100-user-constructor-uninitialized-const-scalar-bad`
- `100-user-constructor-uninitialized-reference-bad`
- `100-user-destructor-anonymous-union-storage`
- `100-using-function-hidden-by-later-member`
- `100-using-reexposes-protected-type`
- `100-zero-width-only-layout`
- `200-implicit-destructor-exception-specification`
- `210-in-class-function-lowir-hint`
- `310-enum-pseudo-destructor-call`
- `400-contiguous-value-initialization-zeroinit`
- `400-functional-cast-to-a-reference-type`
- `400-shared-lexical-return-cleanup`
- `400-zero-initialization-object-boundaries`
- `array-member-empty-paren-mem-initializer`
- `floating-intrinsic-emission-order`
- `local-class-default-member-enclosing-constant`

### course/pa16/controls

- `410-string-literal-readonly`
- `420-lifecycle-inline-policy`
- `430-unreachable-terminator`
- `440-goto-lifetime-cleanup`

### course/pa17/(flat)

- `100-ref-qualified-mixed-cv-overload-bad`
- `100-ref-qualified-mixed-static-overload-bad`
- `100-ref-qualified-rvalue-rank-before-cv-bad`
- `200-braced-constructor-narrowing-bad`
- `200-braced-scalar-constructor-rank`
- `200-out-of-class-defaulted-deleted-assignment-bad`
- `200-out-of-class-defaulted-deleted-constructor-bad`
- `200-out-of-class-defaulted-deleted-destructor-bad`
- `200-out-of-class-defaulted-deleted-move-constructor-bad`
- `200-out-of-class-defaulted-nonspecial-constructor-bad`
- `200-out-of-class-defaulted-wrong-assignment-type-bad`
- `300-further-derived-using-overload-ranking`
- `300-using-directive-function-overload-merge`
- `400-shared-temporary-cleanup-continuation`
- `400-synthesized-constructor-bit-field-units`
- `500-derived-xvalue-binds-const-base-reference`
- `520-direct-class-call-temporary-destination`
- `530-copy-constructor-noalias-boundary`
- `531-move-constructor-noalias-boundary`

### course/pa17/controls

- `532-constructor-alias-boundaries`
- `533-enclosing-temporary-lifetime`
- `533-out-of-class-move-assignment-boundary`
- `534-conditional-copy-elision-permission`
- `535-stable-prefix-query-boundary`
- `536-parameter-object-extent-boundary`
- `rejected-stable-prefix-query-float-index`
- `rejected-stable-prefix-query-no-index`
- `rejected-stable-prefix-query-variadic`
- `rejected-stable-prefix-query-void`

### course/pa18/(flat)

- `100-nonmember-virtual-bad`
- `100-virtual-constructor-bad`
- `200-cv-covariant-return-bad`
- `200-inaccessible-covariant-return-bad`
- `200-invalid-covariant-return-bad`
- `200-static-override-bad`
- `400-shared-return-virtual-destructor-order`
- `400-virtual-deleting-destructor-complete-call`

### course/pa19/(flat)

- `300-class-template-destructor-exception-specification`
- `300-enum-comma-overload-precedes-fallback`
- `300-enum-operator-exact-parameter-filter-bad`
- `300-explicit-instantiation-declaration-after-definition-bad`
- `300-explicit-instantiation-union-key-mismatch-bad`
- `300-explicit-instantiation-unqualified-wrong-namespace-bad`
- `300-pointer-value-init-owned-by-literal`
- `300-unrelated-template-preserves-layout-immediate`
- `310-function-template-lowir-inline-hint`
- `310-unevaluated-static-member-does-not-demand-definition`
- `320-nondependent-template-base`
- `321-incomplete-nondependent-template-base-bad`
- `322-inaccessible-nondependent-template-base-bad`
- `323-qualified-template-parameter-name-collision-bad`
- `324-final-nondependent-template-base-bad`
- `325-union-nondependent-template-base-bad`
- `326-member-template-parameter-name-collision-bad`
- `327-final-class-template-base-bad`
- `328-union-class-template-base-bad`
- `329-forward-class-template-base-bad`
- `330-forward-partial-template-base-bad`
- `331-invalid-nondependent-template-base-body-bad`
- `332-dependent-value-direct-initializer`
- `333-direct-initializer-with-call-arguments`
- `local-type-namespace-template-specialization-abi`

### course/pa20/(flat)

- `100-phase7-integer-literal-type-retained`
- `100-signed-constant-add-overflow-bad`
- `100-signed-constant-divide-overflow-bad`
- `100-signed-constant-left-shift-overflow-bad`
- `100-signed-constant-modulo-overflow-bad`
- `100-signed-constant-multiply-overflow-bad`
- `100-signed-constant-subtract-overflow-bad`
- `100-signed-constant-unary-overflow-bad`
- `300-explicit-specialization-destructor-exception-specification`
- `320-many-function-template-pack-partitions`
- `braced-constructor-function-pack`

### course/pa21/(flat)

- `300-constexpr-delegating-constructor-use`
- `300-constexpr-member-implicit-const`
- `300-constexpr-noexcept-owned-facts`
- `300-constexpr-protected-base-constructor`
- `300-constexpr-recursive-arrow-member-call`
- `300-constexpr-repeated-base-subobject-call-key`
- `300-destructor-noexcept-completed-class`
- `300-local-static-declaration-identity`
- `300-local-static-dynamic-destructor`
- `300-namespace-constant-initialization-constexpr-call`
- `300-nonconstant-constexpr-call-dynamic-init`
- `300-static-constexpr-call-demand`
- `301-constexpr-constructor-instantiating-initializer`
- `400-automatic-constexpr-array-template`
- `500-constexpr-class-return-dangling-member-bad`
- `500-constexpr-global-mutation-bad`
- `500-constexpr-member-owner-nonliteral-bad`
- `500-constexpr-second-base-nonliteral-bad`
- `500-constexpr-variable-nonliteral-bad`
- `500-static-constant-definition-reinitializer-bad`
- `address-of-incomplete-reference`
- `noexcept-in-suppressed-specialization`
- `recursive-dependent-noexcept-deferred`

### course/pa22/(flat)

- `100-partial-selection-retained-owner`
- `300-member-template-head-identity`
- `340-pack-dependent-template-id`
- `345-dependent-qualified-pack-type-requires-typename-bad`
- `346-dependent-typename-valid`
- `347-noncurrent-specialization-requires-typename-bad`
- `348-dependent-member-template-type-requires-typename-bad`
- `349-out-of-class-return-requires-typename-bad`
- `360-qualified-member-template-hiding`
- `361-qualified-variable-template-hiding-bad`
- `362-using-base-template-replay-per-specialization`
- `363-dependent-variable-template-value-argument`
- `364-dependent-completion-tolerates-shape-only-member`
- `365-shape-completion-skips-member-layout`
- `366-explicit-instantiation-member-type-scope`
- `367-member-template-parameter-type-over-class-parameter`
- `368-explicit-instantiation-member-type-access`
- `369-anonymous-member-injection-in-class-template`
- `370-conditional-explicit-on-a-constructor-template`
- `371-variable-template-as-a-nontype-argument`
- `372-short-circuit-does-not-devalue-a-variable-template`
- `373-sfinae-discards-an-ambiguous-operator`
- `374-ambiguous-operator-outside-substitution-bad`
- `375-template-template-argument-in-an-explicit-specifier`
- `376-sizeof-pack-in-a-partial-specialization-shape`
- `377-retained-call-through-a-dependent-qualifier`
- `378-nested-argument-dependent-lookup-keeps-its-classes`
- `379-a-candidate-is-not-collected-twice`

### course/pa23/(flat)

- `300-bad-fixed-qualifier-dependent-call`
- `300-compound-assignment-complete-object-pointer-sfinae`
- `300-declaration-only-member-constructor-demand`
- `300-decltype-prefix-base-identity`
- `300-dependent-parameter-declarator-owner`
- `300-function-template-default-redeclaration-context`
- `300-function-template-exception-demand`
- `300-function-template-result-first-lookup`
- `300-nonconstexpr-contextual-conversion-bad`
- `300-static-constant-storage-demand`
- `300-variable-template-partial-substitution-replay`
- `301-dependent-new-partial-replay`
- `310-adl-overload-set-argument-deduction`
- `320-many-dependent-result-identities`
- `330-deduced-class-variable-is-initialized`

### course/pa25/(flat)

- `100-bad-captureless-lambda-implicit-this`
- `100-bad-cv-auto-function-rvalue-reference-lvalue`
- `100-bad-cv-auto-rvalue-reference-lvalue`
- `100-bad-duplicate-reference-capture`
- `100-cv-auto-runtime-initializer`
- `100-recursive-template-auto-return-bad`
- `100-volatile-auto-runtime-initializer`
- `200-captureless-lambda-empty-parameter-pack`
- `200-captureless-lambda-lexical-private-access`
- `200-conversion-operator-modifiable-reference-selection`
- `200-derived-direct-parameter-boundary-call`
- `200-local-aggregate-array-copy-and-braces`
- `200-overloaded-function-lambda-identity`
- `200-range-for-array-reference-single-evaluation`
- `200-range-for-array-value-categories`
- `200-range-for-iteration-temporary-lifetime`
- `200-template-auto-return-canonical-cache`
- `cached-lambda-rebinds-automatic-capture`
- `lambda-captures-this-for-out-of-class-member-call`
- `lambda-implicitly-captures-this-for-member-call`
- `lambda-inherits-friend-class-access`
- `lambda-local-static-per-specialization`

### course/pa26/(flat)

- `200-constructor-unwind-shares-generated-suffix`
- `210-constructor-early-return-cleanup-region`
- `300-dynamic-cast-inaccessible-upcast-bad`
- `300-dynamic-cast-removes-cv-bad`
- `300-rtti-canonical-type-categories`
- `300-rtti-evaluated-call-demand`
- `400-handler-context-cleanup-continuation`
- `noreturn-direct-object-fallback`

### course/pa27/(flat)

- `300-ambiguous-member-pointer-object-bad`

### course/pa28/(flat)

- `300-ambiguous-virtual-diamond-final-overrider-bad`
- `300-virtual-base-destructor-exception-specification`
- `310-lifecycle-base-entry-inline-policy`

### course/pa29/behavior

- `atomic-load-result-pressure`
- `bounded-temporary-reload-forwarding`
- `branch-local-parameter-spill`
- `call-result-branch-across-call`
- `call-result-index-store-address`
- `caller-saved-binary-reuse-clobber`
- `caller-saved-index-reuse-clobber`
- `compact-memory-displacement-boundaries`
- `constant-byte-store-coalescing`
- `cost-directed-small-zeroinit`
- `dead-address-copy-index-load-folding`
- `dead-address-copy-index-store-folding`
- `dead-address-copy-load-folding`
- `dead-address-copy-store-folding`
- `dead-address-load-folding`
- `dead-address-store-folding`
- `dead-copy-store-folding`
- `deferred-address-parameter-carrier-reuse`
- `explicit-integer-extension-spelling`
- `extended-stack-object-register-source-clobber`
- `extended-stack-scalar-register-source-clobber`
- `f80-compare-branch-pressure`
- `flag-safe-zero-materialization`
- `folded-narrow-load-normalization`
- `forwarded-parameter-load-pressure`
- `forwarded-r8-indirect-call-target`
- `frame-copy-and-constant-multiply-encoding`
- `frame-dividend-direct-return`
- `fused-integer-normalization-encoding`
- `general-constant-remainder-and-unsigned-division`
- `i128-binary-caller-saved-clobber`
- `i128-compare-value-pressure`
- `i128-direct-compare-branch-predicates`
- `i128-multiply-caller-saved-clobber`
- `immediate-frame-reload-forwarding`
- `indirect-second-load-direct-compare-pressure`
- `literal-branch-identity-isolation`
- `loop-invariant-parameter-across-call`
- `loop-invariant-temporary-home`
- `multiple-return-shared-epilogue`
- `narrow-frame-reload-forwarding`
- `narrow-parameter-zero-compare`
- `narrow-signed-frame-to-f80`
- `narrow-zero-extension-encoding`
- `negative-power-of-two-division`
- `noncontiguous-loop-frame-home-lifetime`
- `o2-one-past-local-callee-save`
- `object-load-caller-saved-clobber`
- `one-instruction-temporary-reload-forwarding`
- `optimized-constant-division-result-register`
- `optimized-general-constant-division`
- `parameter-home-address-clobber`
- `phi-frame-address-rematerialization`
- `promoted-rsi-after-object-copy`
- `rax-call-result-intervening-address`
- `redundant-u32-normalization-encoding`
- `remapped-zero-index-parameter`
- `scalar-decay-spill-home-lifetime`
- `scalar-spill-home-copy-lifetime`
- `scalar-to-i128-fixed-register-clobber`
- `scratch-carried-frame-reloads`
- `signed-constant-division-magic`
- `signed-power-of-two-division`
- `single-use-temporary-home-forwarding`
- `small-copyobj-direct-stores`
- `stack-argument-register-clobber`
- `store-frame-address-value`
- `transient-scratch-address-folding`
- `typed-literal-payloads`
- `unsigned-power-of-two-division`
- `wide-compare-narrow-frame-value`
- `wide-forwarded-call-argument-home`
- `wide-live-parameter-temporary-pressure`
- `wide-parameter-home-clobbers-incoming`
- `wide-parameter-loop-exit-spill`
- `wide-parameter-register-preservation`
- `wide-promoted-parameter-call-clobber`
- `zero-compare-test-encoding`

### course/pa29/controls

- `900-power-of-two-multiply`
- `901-structured-factor-multiply`
- `902-frame-copy-operands`
- `903-direct-global-storage`
- `904-o0-layout-policy`
- `905-small-copy-boundary`
- `906-aligned-medium-copy-direct`
- `907-weakly-aligned-medium-copy-compact`
- `908-aligned-large-copy-direct`
- `909-oversized-aligned-copy-compact`
- `910-large-switch-immediate-cases`
- `911-strlen-prefix-call`
- `912-strlen-prefix-declaration`
- `913-strlen-prefix-incompatible`
- `914-copyobj-indexed-parameter-order`
- `915-unused-result-builtin-memcpy`
- `916-unreachable-terminator`
- `918-by-address-call-materializes-storage`
- `919-zero-extending-and-mask`

### course/pa29/strict

- `copied-compare-result-across-call`
- `fallthrough-jump-encoding`
- `immediate-move-encoding-boundaries`
- `scalar-copy-location-sharing`

### course/pa29/structural

- `direct-call-result-consumers`
- `direct-local-global-storage-address`
- `direct-return-placement`
- `discarded-slots-do-not-reserve-frame`
- `fixed-shift-count-operands`
- `forwarded-parameter-extended-call-placement`
- `i128-direct-compare-branch`
- `immediate-memory-stores`
- `incoming-parameter-emitted-clobbers`
- `index-address-placement`
- `indexed-memory-addressing`
- `integer-memory-right-operands`
- `known-normalized-integer-producers`
- `narrow-call-result-terminal-consumers`
- `native-layout-policy-o0`
- `nonadjacent-object-result-frame-placement`
- `phi-critical-loop`
- `phi-force-inline-predecessor`
- `phi-parallel-cycle`
- `representation-preserving-copy-placement`
- `single-block-call-argument-coalescing`
- `single-edge-callee-saved-retention`
- `storage-only-derived-address-placement`
- `typed-i128-high-word`
- `unused-index-address-elision`

### course/pa30/(flat)

- `300-runtime-constructor-call-unwind-cleanup`
- `300-runtime-destructor-function-try-block`
- `300-runtime-eh-handler-preserves-this`
- `300-runtime-foreign-relocation-object-alias`
- `300-runtime-imported-function-address`
- `300-runtime-left-nested-short-circuit-temporary-lifetime`
- `300-runtime-stack-argument-call-unwind-cleanup`
- `300-runtime-statement-expression-unreachable-result`
- `310-anonymous-template-specialization-linkage`
- `310-main-local-type-abi-context`
- `320-runtime-i128-direct-condition`
- `330-runtime-inlined-cleanup-during-unwind`

### course/pa31/(flat)

- `300-host-eh-multiple-typed-catches`
- `310-shared-conditional-cleanup-resume`
- `320-conditional-temporary-cleanup-across-try`
- `330-host-eh-nested-catch-forward-at-o1`
- `340-unwind-resume-coalescing-barrier`
- `350-runtime-builtin-and-foreign-memcpy-object-alias`
- `360-adjacent-lsda-call-site-coalescing`
- `370-lsda-distinct-cleanup-landings`
- `380-lsda-distinct-catch-actions`
- `390-shared-cleanup-suffix-landing-entry`
- `400-shared-terminate-action`
- `410-shared-resume-after-stack-call-cleanup`
- `420-sparse-unprotected-lsda-coverage`

### course/pa32/(flat)

- `200-anonymous-storage-owner-initialization`
- `200-canonical-template-type-dag-scaling`
- `200-dependent-abi-multiword-type-argument`
- `200-dependent-member-alias-symbol-spelling`
- `200-explicit-function-specialization-cross-object`
- `200-extern-template-member-template-instantiation`
- `200-host-extern-template-vtable-reference`
- `200-local-object-fixup-resolution`
- `200-local-type-template-discriminator-runtime`
- `200-member-conversion-nttp-symbol-spelling`
- `200-member-operator-nttp-symbol-spelling`
- `200-member-template-nttp-symbol-spelling`
- `200-member-template-pack-nttp-symbol-spelling`
- `200-unused-inline-local-static-closure`
- `unreachable-internal-function-pruning`
- `weak-function-body-comdat`

### course/pa32/controls

- `section-conflicting-redeclaration`
- `section-token-safe`
- `section-token-unsafe`

### course/pa33/(flat)

- `200-multi-abi-tag-member-publication`
- `201-invalid-abi-tag-argument`
- `202-gnu-function-effect-attributes`

### course/pa34/compile

- `200-out-of-class-defaulted-assignment-traits`
- `201-invalid-defaulted-nonspecial-assignment`
- `202-int128-to-floating-native`
- `203-builtin-integer-intrinsic-arity-bad`
- `204-gnu-aligned-dependent-argument`
- `205-hosted-type-specifier-functional-cast`
- `206-atomic-member-is-a-literal-subobject`

### course/pa34/run

- `900-atomic-noexcept-has-no-terminate-boundary`

### course/pa35/compile

- `200-dependent-composite-template-shape`
- `214-local-callable-cross-function-reference-negative`
- `214-local-callable-lifetime-boundary`
- `215-static-data-member-addressability`
- `215-zero-initialized-bulk-member`
- `300-qualified-owner-identity`
- `400-evaluated-retained-call-demand-bad`
- `400-explicit-id-pack-index-out-of-range`
- `400-reachable-missing-return-bad`
- `400-retained-special-member-exception-identity`
- `400-retained-special-member-exception-mismatch-bad`
- `400-retained-type-as-value-stays-invalid`
- `400-static-member-retained-no-viable-bad`
- `420-base-same-type-alias-ambiguous-bad`
- `420-constant-left-shift-width-bad`
- `420-hosted-fixed-vector-builtin-arity-bad`
- `420-hosted-fixed-vector-builtin-type-bad`
- `420-hosted-using-function-distinct-entity-bad`
- `420-namespace-distinct-type-alias-ambiguous-bad`
- `500-parameter-owned-noexcept-scope`
- `600-aggregate-class-reference-member`
- `600-basic-string-implicit-destructor-noexcept`
- `600-direct-member-call-pack-expansion`
- `600-distinct-float128-specializations`
- `600-empty-variadic-tail`
- `600-explicit-id-deduces-trailing-pack`
- `600-explicit-instantiation-nontype-argument`
- `600-fixed-reference-beats-forwarding-pack`
- `600-function-template-reentrant-result-sfinae`
- `600-initializer-list-alias-reentry`
- `600-native-object-value-transport`
- `600-nested-lambda-cleanup-boundary`
- `600-noreturn-control-convergence`
- `600-reference-trait-conversion-boundaries`
- `600-retained-injected-friend-tag`
- `600-specialization-owned-retained-call`
- `600-static-downcast-completes-specialization`
- `600-static-member-retained-replay`
- `600-template-argument-function-type`
- `600-transitive-anonymous-member-array-init`
- `600-unevaluated-retained-call-demand`
- `620-braced-deferred-specialization`
- `620-class-conditional-enclosing-cleanup`
- `620-extended-call-result-pressure`
- `620-hosted-fixed-vector-builtins`
- `620-hosted-using-function-same-entity-roundtrip`
- `620-initializer-list-shell-copy`
- `620-namespace-same-type-alias-convergence`
- `620-unsigned-constant-left-shift-modulo`
- `630-hosted-local-lambda-vector-abi-prefix`

### course/pa36/link

- `700-closed-builtin-alias-collision`

### course/pa37/debuginfo/driver/o3

- `500-source-full-unroll-debug`

### course/pa37/debuginfo/o1

- `410-keep-loop-location`

### course/pa37/debuginfo/o3

- `500-full-unroll-debug`

### course/pa37/driver/o1

- `410-source-noinline-metadata`
- `415-source-lambda-noinline-metadata`
- `416-source-call-only-internal-pruned`
- `420-source-unreachable-terminator`
- `425-source-inline-hint-late-nonleaf`
- `430-source-lifecycle-base-entry-inline`
- `440-source-landing-cleanup-inline`
- `450-source-gnu-function-effects`
- `455-source-optimize-predefine`
- `460-source-copy-constructor-noalias`
- `461-source-move-constructor-noalias`

### course/pa37/driver/o3

- `500-source-full-unroll`

### course/pa37/o0

- `500-p31-transforms-disabled`

### course/pa37/o1

- `300-i128-zext-add-carry`
- `320-floating-nan-compare`
- `340-share-terminal-resume`
- `350-share-exact-cleanup-tail`
- `360-inline-after-callee-simplify`
- `370-bottom-up-no-unwind-wrapper`
- `370-inline-moved-phi-predecessor`
- `375-inline-callful-void-wrapper-phi-edge`
- `380-inline-generated-block-state`
- `385-branch-boolean-conversion`
- `386-negated-boolean-compare`
- `387-duplicate-block-loads`
- `388-staged-copy-forwarding`
- `389-aggregate-slot-scalar-replacement`
- `390-inline-growth-budget`
- `390-sink-cold-blocks`
- `391-inline-growth-budget-boundary`
- `392-inline-trivial-leaf-budget-exempt`
- `392-sink-cold-only-definitions`
- `395-truncate-noreturn-continuation`
- `400-bottom-up-prune-unreachable`
- `405-inline-no-unwind-object-inside-eh`
- `410-loop-invariant-scalar`
- `420-loop-invariant-safety`
- `430-nested-and-multiple-exit-licm`
- `440-bounded-loop-preheader`
- `450-dominator-scoped-expression-reuse`
- `460-unreachable-terminator-branch`
- `470-inline-discardable-single-call`
- `475-inline-discardable-size-cap`
- `476-inline-single-call-caller-budget`
- `480-post-prune-called-once-cascade`
- `485-inline-throwing-wrapper-inside-eh`
- `486-direct-throw-callee-not-inlined`
- `487-inline-cleanup-convergence`
- `490-inline-hint-late-nonleaf`
- `495-region-granular-dead-eh-strip`
- `496-region-strip-publishes-no-unwind`
- `497-inline-no-unwind-landing-helper`
- `498-inline-inferred-no-unwind-landing-chain`
- `499-retain-unsafe-landing-calls`
- `500-readonly-call-dce`
- `501-forward-only-block-merge`
- `502-zero-index-canonicalization`
- `503-boolean-phi-branch`
- `504-edge-known-branch`
- `506-nonzero-underflow-predicate`
- `507-adjacent-noalias-scalar-copy`
- `508-fully-overwritten-zero-init`
- `509-shared-loop-inline-policy`
- `510-cold-path-discounted-inlining`
- `511-phi-integrity-survivors`

### course/pa37/o2

- `300-i128-zext-add-carry`
- `320-floating-nan-compare`
- `330-forward-block-slot-promotion`
- `410-phi-slot-promotion`
- `420-pruned-phi-placement`
- `430-phi-forward-simplification`
- `440-incomplete-phi-dependency`
- `450-loop-invariant-memory`
- `460-counted-loop-simplification`
- `470-memory-value-numbering`
- `480-memory-value-numbering-barriers`
- `481-memory-gvn-eh-regions`
- `490-partial-redundancy-elimination`
- `500-pre-insertion-budget`
- `510-pre-exception-barrier`
- `520-interprocedural-argument-specialization`
- `525-pre-address-result-type`
- `526-promote-type-changing-copy`
- `527-promote-normalizes-phi-inputs`
- `528-small-object-scalar-replacement`
- `529-memory-gvn-pointer-identity`
- `540-volatile-access-preservation`
- `escaped-slot-store-after-address`

### course/pa37/o3

- `530-full-unroll-small-constant-loops`
- `540-unroll-linear-1`
- `541-unroll-linear-2`
- `542-unroll-linear-4`
- `550-late-inline-after-scalar-replacement`
- `560-late-inline-callful-multiblock`
- `565-late-inline-complex-size-cap`
- `570-diamond-value-numbering`
- `571-diamond-select-identities`

### course/pa37/object-roundtrip

- `400-default-no-optimization`
- `410-debug-local-literal-canonicalization`
- `420-force-inline-ordered-blocks`
- `430-bottom-up-pruned-internal-chain`
- `440-phi-slot-promotion`
- `450-pre-address-result-type`
- `460-canonical-comparison-result`
- `470-inferred-no-unwind-object-eh`
- `480-inline-hint`
- `490-lifecycle-base-entry-inline`
- `500-unreachable-terminator`
- `510-gnu-section-attribute`
- `520-conditional-copy-elision-replay`
- `530-stable-prefix-query-replay`
- `540-parameter-object-extent-replay`
- `interprocedural-argument-specialization`
- `o3-full-unroll`

### course/pa38/behavior/o1

- `320-indexed-dead-setup-load`
- `405-deferred-compare-across-call`
- `405-deferred-compare-across-call.ref`
- `430-rematerialized-slot-address-consumers`
- `430-rematerialized-slot-address-consumers.ref`

### course/pa38/behavior/o2

- `300-reused-frame-home-layout-lifetime`
- `320-indexed-dead-setup-load`
- `400-branch-spill-register-home`
- `400-branch-spill-register-home.ref`
- `410-cyclic-edge-register-pressure`
- `410-cyclic-edge-register-pressure.ref`
- `420-retained-loop-invariant-reactive-pressure`
- `420-retained-loop-invariant-reactive-pressure.ref`

### course/pa38/behavior/o3

- `500-o3-unrolled-lowir-behavior`

### course/pa38/debuginfo/o1

- `200-identity-move-debug`
- `200-identity-move-debug.ref`

### course/pa38/debuginfo/o2

- `200-edge-placement-debug`
- `200-edge-placement-debug.ref`

### course/pa38/debuginfo/o3

- `500-o3-machine-debug`

### course/pa38/driver

- `440-function-census`
- `441-loop-census`

### course/pa38/o1

- `300-call-result-across-eh-push`
- `400-identity-move-cleanup`
- `400-identity-move-cleanup.ref`
- `406-volatile-access-emission`
- `406-volatile-access-emission.ref`
- `420-loop-and-eh-placement`
- `425-native-layout-policy-guards`
- `426-staged-home-selection`
- `427-rematerialized-storage-addresses`
- `427-rematerialized-storage-addresses.ref`
- `428-dominated-post-call-use-tails`

### course/pa38/o2

- `300-call-result-across-eh-push`
- `400-loop-invariant-call-crossing-placement`
- `400-loop-invariant-call-crossing-placement.ref`
- `410-eh-edge-placement-barrier`
- `410-eh-edge-placement-barrier.ref`

### course/pa38/o3

- `500-o3-machine-pipeline`
- `500-o3-machine-pipeline.ref`

### regression/pa29/strict

- `100-copyobj`
- `100-direct-call-branch`
- `100-object-abi-lowered`
- `100-ret0`
- `100-startup-shutdown-hooks`
- `100-structured-global-data`
- `100-switch-terminator`
- `100-zeroinit`
- `200-bad-missing-terminator`
- `200-class-constructor-member-init`
- `200-class-template-field`
- `200-f80-direct-call`
- `200-f80-structured-global-data`
- `200-indirect-call-six-register-args`
- `200-non-type-class-specialization`
- `200-pass-by-value-lvalue`
- `200-pcrel-global-data-load`
- `200-trivial-param-slot-promotion`
- `300-atomic-add-fetch`
- `300-atomic-compare-exchange-failure`
- `300-atomic-compare-exchange-success`
- `300-atomic-exchange`
- `300-atomic-load-store`
- `300-atomic-seq-cst-fence`
- `300-unsigned-compare-predicates`
- `300-unsigned-int-ops`
- `400-u32-bswap-and-float-conversions`
- `600-atomic-i32-exchange`
- `600-atomic-i32-seqcst-store`
- `600-call-pass-mode-address-materialization`
- `600-call-pass-mode-register-temp-address-materialization`
- `600-direct-object-return-temp-padding`
- `600-readonly-global-extra-section-runtime`
- `600-thread-local-direct-native-runtime`
- `800-atomic-i128-compare-exchange`
- `900-slot-address-stack-call-argument`
- `copied-compare-result-across-call`
- `fallthrough-jump-encoding`
- `immediate-move-encoding-boundaries`
- `scalar-copy-location-sharing`

### regression/pa29/structural

- `200-f64-direct-call`
- `200-stack-arguments-beyond-six`
- `300-integral-float-conversions`
- `300-promoted-i32-compare-adjacent-pointer`
- `400-call-clobber-register-pressure`
- `400-f32-leaf-register-chain`
- `400-f32-ordered-compare-branch`
- `400-f64-eq-compare-branch`
- `400-f64-leaf-copy-chain`
- `400-float-width-conversions`
- `400-i32-direct-compare-branch`
- `400-i64-leaf-register-chain`
- `400-u32-compare-value-materialize`
- `400-u32-direct-compare-branch`
- `500-const-ptr-null-direct-compare-branch`
- `500-f64-compare-value-materialize`
- `500-f80-arithmetic-compare-owner`
- `500-i16-leaf-normalize-chain`
- `500-i64-direct-compare-branch`
- `500-i8-direct-compare-branch`
- `500-mixed-gpr-xmm-call-abi`
- `500-ptr-diff-switch`
- `500-ptr-index-arithmetic`
- `500-ptr-null-direct-compare-branch`
- `500-u16-direct-compare-branch`
- `500-u32-f64-conversion-branch`
- `600-atomic-i8-load-store`
- `600-floating-short-circuit-branch`
- `600-i8-signed-frame-load-widen`
- `600-indirect-mixed-gpr-xmm-call-abi`
- `600-ptr-compare-value-materialize`
- `600-short-circuit-and-call-diamond`
- `600-short-circuit-or-call-diamond`
- `600-u16-zero-frame-load-widen`
- `600-unary-not-call-branch`
- `700-call-pass-mode-address-materialization`
- `700-call-setup-forwarding-no-preserve`
- `700-direct-branch-source-live-across-call`
- `700-f64-f80-implicit-store-return-convert`
- `700-i8-signed-global-load-widen`
- `700-index-chain-register-reuse`
- `700-object-call-result-slot-alias`
- `700-object-param-slot-alias`
- `700-u16-zero-global-load-widen`
- `800-conditional-edge-liveness`
- `800-direct-branch-call-slot-ptr-compare`
- `800-direct-branch-postdec-slot-reload`
- `800-direct-branch-slot-address-compare`
- `800-f32-return-ret-liveness`
- `800-forwarded-param-identity-live-across-call`
- `800-global-pointer-address-call`
- `800-loop-slot-compare-reload`
- `800-runtime-f64-decimal-literal-rounding`
- `800-runtime-i64-large-alu-immediate`
- `800-runtime-zero-only-global-pointer-alignment`
- `800-slot-address-rematerialization`
- `800-slot-backed-direct-compare-branch`
- `800-switch-call-case-liveness`
- `800-xmm-live-across-integer-call`
- `900-symbolic-global-call-argument`
- `direct-call-result-consumers`
- `direct-local-global-storage-address`
- `direct-return-placement`
- `discarded-slots-do-not-reserve-frame`
- `fixed-shift-count-operands`
- `forwarded-parameter-extended-call-placement`
- `i128-direct-compare-branch`
- `immediate-memory-stores`
- `incoming-parameter-emitted-clobbers`
- `index-address-placement`
- `indexed-memory-addressing`
- `integer-memory-right-operands`
- `known-normalized-integer-producers`
- `narrow-call-result-terminal-consumers`
- `native-layout-policy-o0`
- `nonadjacent-object-result-frame-placement`
- `phi-critical-loop`
- `phi-force-inline-predecessor`
- `phi-parallel-cycle`
- `representation-preserving-copy-placement`
- `single-block-call-argument-coalescing`
- `single-edge-callee-saved-retention`
- `storage-only-derived-address-placement`
- `typed-i128-high-word`
- `unused-index-address-elision`

### regression/pa37/controls

- `520-inline-limit-once-cap`
- `521-partial-inline-census`
- `522-driver-inline-limit`
- `523-loop-local-boolean-phi-branch`
- `524-post-prune-inline-slot-promotion`
- `525-historical-lowir-contracts`
- `526-late-inline-hint-load-reuse`
- `527-readonly-byte-strlen`
- `528-nonzero-underflow-value-proof`
- `529-loop-carried-store-forwarding`
- `530-post-inline-memory-gvn`
- `531-weak-specialization-profitability`
- `532-o3-group-specialization`
- `533-o3-repeat-stable-query`
- `534-o3-loop-priority-inline`
- `535-o3-terminal-phi-return`
- `536-zero-bounded-signed-range`
- `537-o3-addressed-scalar-slot`
- `538-inlined-addressed-store`
- `539-o3-private-table-prefilter`
- `540-copy-elision-permission`
- `540-o3-readonly-string-specialization`
- `541-o3-stable-prefix-query`
- `542-o3-terminal-staged-object-swap`
- `543-o3-bounded-object-memory`
- `544-o3-fast-path-versioning`
- `545-o3-terminal-query-slow-suffix`

### regression/pa37/driver/o1

- `100-cppgm-emit-lowir-builtin-inf-declaration`
- `100-cppgm-emit-lowir-constant-fold`
- `100-cppgm-emit-lowir-local-array-object-slot`
- `410-source-noinline-metadata`
- `415-source-lambda-noinline-metadata`
- `416-source-call-only-internal-pruned`
- `420-source-unreachable-terminator`
- `425-source-inline-hint-late-nonleaf`
- `430-source-lifecycle-base-entry-inline`
- `440-source-landing-cleanup-inline`
- `450-source-gnu-function-effects`
- `455-source-optimize-predefine`
- `460-source-copy-constructor-noalias`
- `461-source-move-constructor-noalias`

### regression/pa37/driver/o2

- `100-cppgm-emit-lowir-add1`
- `100-cppgm-emit-lowir-branch-edge-nonbool-temp-preserved`
- `100-cppgm-emit-lowir-builtin-inf-declaration`
- `100-cppgm-emit-lowir-initlist-backing`
- `100-cppgm-emit-lowir-storage-temp-constptr-guard`
- `100-cppgm-emit-lowir-value-init-class-zero-init`

### regression/pa37/driver/o3

- `500-source-full-unroll`

### regression/pa37/o0

- `100-canonical-roundtrip`
- `100-force-inline-metadata`
- `110-wide-integer-literal`
- `200-floating-special-literals`
- `500-p31-transforms-disabled`

### regression/pa37/o1

- `100-branch-constant-fold`
- `100-cfg-available-expression`
- `100-cfg-join-const-propagation`
- `100-commutative-expression-reuse`
- `100-executable-edge-propagation`
- `100-jump-chain-collapse`
- `100-local-const-fold-dce`
- `100-local-cse`
- `100-straight-line-merge`
- `100-unused-call-preserved`
- `200-boolean-compare-cleanup`
- `200-cmp-direction-expression-reuse`
- `200-eh-addr-cse-guard`
- `200-eh-cleanup-addr-rematerialize`
- `200-eh-handler-reachability`
- `200-eh-landingpad-merge-guard`
- `200-identity-convert-cleanup`
- `200-local-integer-reassociation`
- `200-pointer-expression-reuse`
- `200-readnone-call-dce`
- `200-slot-load-across-successors-not-promoted`
- `300-i128-zext-add-carry`
- `300-inline-branching-scalar-return-merge`
- `300-inline-call-dest-collision`
- `300-inline-direct-object-parameter-copy`
- `300-inline-nested-multiblock-continuation`
- `300-inline-prefer-local-branchy-accessor`
- `300-inline-prefer-local-hash-helper`
- `300-small-branching-indirect-result-inline`
- `300-small-direct-call-inline`
- `300-type-changing-copy-preserved`
- `320-floating-nan-compare`
- `340-share-terminal-resume`
- `350-share-exact-cleanup-tail`
- `360-inline-after-callee-simplify`
- `370-bottom-up-no-unwind-wrapper`
- `370-inline-moved-phi-predecessor`
- `375-inline-callful-void-wrapper-phi-edge`
- `380-inline-generated-block-state`
- `385-branch-boolean-conversion`
- `386-negated-boolean-compare`
- `387-duplicate-block-loads`
- `388-staged-copy-forwarding`
- `389-aggregate-slot-scalar-replacement`
- `390-inline-growth-budget`
- `390-sink-cold-blocks`
- `391-inline-growth-budget-boundary`
- `392-inline-trivial-leaf-budget-exempt`
- `392-sink-cold-only-definitions`
- `395-truncate-noreturn-continuation`
- `400-bottom-up-prune-unreachable`
- `400-cross-block-no-unwind-eh-strip`
- `400-dead-slot-merge-diamond-dce`
- `400-dead-slot-merge-side-effect-preserved`
- `400-inline-branching-no-unwind-inside-eh-region`
- `400-inline-branching-object-return-merge-slot`
- `400-inline-eh-bearing-callee-at-depth-zero-blocked`
- `400-inline-inferred-nothrow-inside-eh-region`
- `400-inline-landingpad-incoming-eh-edge-guard`
- `400-inline-multireturn-merge-slot-name-collision`
- `400-inline-mutual-recursive-callee-blocked`
- `400-inline-no-unwind-wrapper-eh-strip`
- `400-inline-object-return-copy-consumer`
- `400-inline-object-return-inside-eh-region`
- `400-inline-recursive-chain-callee-blocked`
- `400-inline-scalar-inside-eh-region`
- `400-inline-second-round-eh-wrapper`
- `400-inline-self-recursive-callee-blocked`
- `400-inline-site-id-skips-existing-generated-name`
- `405-inline-no-unwind-object-inside-eh`
- `410-loop-invariant-scalar`
- `420-loop-invariant-safety`
- `430-nested-and-multiple-exit-licm`
- `440-bounded-loop-preheader`
- `450-dominator-scoped-expression-reuse`
- `460-unreachable-terminator-branch`
- `470-inline-discardable-single-call`
- `475-inline-discardable-size-cap`
- `476-inline-single-call-caller-budget`
- `480-post-prune-called-once-cascade`
- `485-inline-throwing-wrapper-inside-eh`
- `486-direct-throw-callee-not-inlined`
- `487-inline-cleanup-convergence`
- `490-inline-hint-late-nonleaf`
- `495-region-granular-dead-eh-strip`
- `496-region-strip-publishes-no-unwind`
- `497-inline-no-unwind-landing-helper`
- `498-inline-inferred-no-unwind-landing-chain`
- `499-retain-unsafe-landing-calls`
- `500-effect-free-loop-deleted`
- `500-effect-free-loop-deleted-exit-phi`
- `500-effect-free-loop-deleted-twin-backward`
- `500-effect-free-loop-kept-live-out`
- `500-effect-free-loop-kept-spin`
- `500-effect-free-loop-kept-store`
- `500-readonly-call-dce`
- `501-forward-only-block-merge`
- `502-zero-index-canonicalization`
- `503-boolean-phi-branch`
- `504-edge-known-branch`
- `506-nonzero-underflow-predicate`
- `507-adjacent-noalias-scalar-copy`
- `508-fully-overwritten-zero-init`
- `509-shared-loop-inline-policy`
- `510-cold-path-discounted-inlining`
- `510-fill-loop-byte-from-reference`
- `510-fill-loop-kept-first-last`
- `510-fill-loop-twin-phi-position-published`
- `510-fill-loop-word-from-reference-guarded`
- `510-fill-loop-word-pattern`
- `510-fill-loop-word-runtime-value`
- `510-fill-loop-word-zero`
- `511-phi-integrity-survivors`
- `530-conditional-address-load-forwarded`
- `540-block-store-load-forwarding-with-eh`
- `541-phi-aware-jump-only-bypass`
- `542-edge-known-branch-chain`
- `543-constant-terminal-with-phis`
- `544-definition-order-after-fold`
- `545-phi-aware-block-merge`
- `546-late-aggregate-split-after-forwarding`
- `547-folded-copy-of-exception-callee`
- `548-unsigned-compare-against-zero`

### regression/pa37/o2

- `100-atomic-add-fetch-value-use`
- `100-eh-handler-reachability`
- `100-eh-landingpad-merge-guard`
- `100-eh-promoted-slot-store-live-on-catch`
- `100-eh-slot-merge-promotion-guard`
- `100-pointer-slot-promotion`
- `100-promoted-slot-dead-store`
- `100-slot-loop-prune`
- `100-slot-promotion`
- `100-slot-promotion-address-escape`
- `100-switch-case-value-propagation`
- `100-switch-constant-fold`
- `300-i128-zext-add-carry`
- `320-floating-nan-compare`
- `330-forward-block-slot-promotion`
- `400-hoist-loads-from-store-free-loop`
- `410-phi-slot-promotion`
- `420-pruned-phi-placement`
- `430-phi-forward-simplification`
- `440-incomplete-phi-dependency`
- `450-loop-invariant-memory`
- `460-counted-loop-simplification`
- `470-memory-value-numbering`
- `480-memory-value-numbering-barriers`
- `481-memory-gvn-eh-regions`
- `490-partial-redundancy-elimination`
- `500-pre-insertion-budget`
- `510-pre-exception-barrier`
- `520-interprocedural-argument-specialization`
- `525-pre-address-result-type`
- `526-promote-type-changing-copy`
- `527-promote-normalizes-phi-inputs`
- `528-small-object-scalar-replacement`
- `529-memory-gvn-pointer-identity`
- `540-volatile-access-preservation`
- `escaped-slot-store-after-address`

### regression/pa37/o3

- `530-full-unroll-small-constant-loops`
- `540-unroll-linear-1`
- `541-unroll-linear-2`
- `542-unroll-linear-4`
- `550-late-inline-after-scalar-replacement`
- `560-late-inline-callful-multiblock`
- `565-late-inline-complex-size-cap`
- `570-diamond-value-numbering`
- `571-diamond-select-identities`

### regression/pa38/behavior/o1

- `320-indexed-dead-setup-load`
- `405-deferred-compare-across-call`
- `405-deferred-compare-across-call.ref`
- `430-rematerialized-slot-address-consumers`
- `430-rematerialized-slot-address-consumers.ref`

### regression/pa38/behavior/o2

- `300-reused-frame-home-layout-lifetime`
- `320-indexed-dead-setup-load`
- `400-branch-spill-register-home`
- `400-branch-spill-register-home.ref`
- `410-cyclic-edge-register-pressure`
- `410-cyclic-edge-register-pressure.ref`
- `420-retained-loop-invariant-reactive-pressure`
- `420-retained-loop-invariant-reactive-pressure.ref`

### regression/pa38/behavior/o3

- `500-o3-unrolled-lowir-behavior`

### regression/pa38/controls

- `442-acyclic-phi-frame-home`
- `443-cyclic-choice-region-residency`
- `444-call-free-fast-loop-phi-residency`
- `445-historical-placement-contracts`
- `446-call-free-callee-save-recoloring`
- `447-adjacent-frame-compare-forwarding`
- `448-local-loop-phi-activation`
- `449-conditional-fallthrough-layout`
- `450-deferred-carrier-lifetime-reset`
- `451-call-result-plan-reservation`
- `452-o3-large-function-alignment`
- `453-medium-copy-direct-chunks`
- `454-composite-copy-pointer-preservation`
- `455-adjacent-integer-normalizations`
- `456-selected-parameter-index-home`
- `457-o3-common-path-memory`
- `458-stable-prefix-boundary-replay`
- `459-o3-parameter-address-rematerialization`
- `460-o3-sibling-tail-transfer`
- `461-o3-scalar-sibling-query`

### regression/pa38/o1

- `100-branch-fallthrough-cleanup`
- `100-call-address-cleanup`
- `100-call-address-cleanup.ref`
- `100-call-argument-immediate-rematerialize`
- `100-call-result-copy-cleanup`
- `100-fallthrough-jump-elision`
- `100-float-copy-coalesce`
- `100-frame-address-fold`
- `100-frame-address-fold.ref`
- `100-return-copy-coalesce`
- `200-bulk-copy-register-setup`
- `200-call-argument-register-copy`
- `200-cross-block-copy-liveness`
- `200-cross-block-copy-liveness.ref`
- `300-call-result-across-eh-push`
- `400-identity-move-cleanup`
- `400-identity-move-cleanup.ref`
- `406-volatile-access-emission`
- `406-volatile-access-emission.ref`
- `420-loop-and-eh-placement`
- `425-native-layout-policy-guards`
- `426-staged-home-selection`
- `427-rematerialized-storage-addresses`
- `427-rematerialized-storage-addresses.ref`
- `428-dominated-post-call-use-tails`

### regression/pa38/o2

- `100-branch-fallthrough-cleanup`
- `100-call-address-cleanup`
- `100-call-address-cleanup.ref`
- `100-call-argument-immediate-rematerialize`
- `100-call-result-copy-cleanup`
- `100-callee-saved-prune`
- `100-callee-saved-prune.ref`
- `100-fallthrough-jump-elision`
- `100-float-copy-coalesce`
- `100-frame-address-fold`
- `100-frame-address-fold.ref`
- `100-jump-trace-layout`
- `100-return-copy-coalesce`
- `200-bulk-copy-register-setup`
- `200-call-argument-register-copy`
- `200-cross-block-copy-liveness`
- `200-cross-block-copy-liveness.ref`
- `300-call-result-across-eh-push`
- `400-loop-invariant-call-crossing-placement`
- `400-loop-invariant-call-crossing-placement.ref`
- `410-eh-edge-placement-barrier`
- `410-eh-edge-placement-barrier.ref`

### regression/pa38/o3

- `500-o3-machine-pipeline`
- `500-o3-machine-pipeline.ref`
