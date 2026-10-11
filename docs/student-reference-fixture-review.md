# Final fixture review

Reviewed inputs since `fb15cd49e`; source bodies and frozen proofs are retained under `/tmp/cppgm-v4-audit-review/fixture-final-review/`. Companion files belong to their driver rather than counting as independent fixtures.

| Input | Disposition | Retained purpose |
| --- | --- | --- |
| `pa10/tests/general/100-reference-cv-array-const-volatile-lvalue.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/100-reference-cv-array-drop-const-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-array-drop-volatile-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-array-multidim-drop-const-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-array-overload.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/100-reference-cv-pointer-deep-const-reference-middle-qualified.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/100-reference-cv-pointer-deep-const-reference-missing-middle-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-pointer-mutable-reference-add-pointee-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-pointer-volatile-reference-add-pointee-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-scalar-const-volatile-lvalue.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/100-reference-cv-scalar-const-volatile-rvalue-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-scalar-volatile-converted-lvalue-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-volatile-array-lvalue.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/100-reference-cv-volatile-array-multidim-lvalue.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/100-reference-cv-volatile-array-pointer-deep-const.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/100-reference-cv-volatile-array-pointer-deep-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-volatile-array-pointer-pointee-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/100-reference-cv-volatile-array-pointer-reference.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/100-reference-cv-volatile-array-pointer.t` | Combine | All successful boundaries retained in `pa10/tests/general/100-reference-cv-qualified-lvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/200-discarded-reference-call-effects.t` | Retain boundary | The id expression and the used reference result both require a read. |
| `pa10/tests/general/200-empty-unknown-bound-array-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/200-known-mutable-array-storage.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa10/tests/general/200-reference-cv-array-const-volatile-xvalue-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa10/tests/general/200-reference-cv-array-const-xvalue.t` | Combine | All successful boundaries retained in `pa10/tests/general/200-reference-cv-array-xvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/200-reference-cv-array-multidim-const-xvalue.t` | Combine | All successful boundaries retained in `pa10/tests/general/200-reference-cv-array-xvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/200-reference-cv-array-mutable-xvalue-const-reference.t` | Combine | All successful boundaries retained in `pa10/tests/general/200-reference-cv-array-xvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/200-reference-cv-volatile-array-rvalue.t` | Combine | All successful boundaries retained in `pa10/tests/general/200-reference-cv-array-xvalue-boundaries.t`; common harness factored where applicable. |
| `pa10/tests/general/300-defined-nonvoid-control-flow.t` | Retain boundary | Syntactic non-void ends need not be reachable on any evaluated path.  A goto can enter a nested label independently of the enclosing condition. |
| `pa11/tests/general/100-local-class-default-member-enclosing-constant.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/100-local-class-enclosing-automatic-use-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/general/100-local-class-enclosing-constant-address-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/general/100-local-class-enclosing-constant-reference-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/general/100-local-class-enclosing-default-argument-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/general/100-nested-class-forward-shadow-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/general/100-static-reference-address-before-constructor.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/200-aggregate-member-class-array-lifetime.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/200-empty-aggregate-member-constructor-argument-effects.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/200-hidden-friend-class-tag-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/general/300-alignas-class-layout.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/300-constructor-array-loop-enclosing-cleanup.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/300-synthesized-array-member-lifecycle.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/400-aggregate-array-volatile-member-initialization.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/400-bit-field-const-volatile-reference-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/general/400-bit-field-integral-promotion.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/general/400-bit-field-mutable-reference-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/general/400-bitfield-aggregate-init.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa11/tests/spec/300-nonstatic-member-out-of-class-definition-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/spec/300-static-member-definition-type-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa11/tests/spec/300-static-member-duplicate-definition-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/100-aggregate-braced-return-member-copy-observations.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/100-copy-init-constructor-explicit-copy.t` | Retain boundary | N3485 8.5, 13.3.1.4: select the conversion, then validate final construction. |
| `pa12/tests/general/100-copy-init-converting-constructor-deleted-copy-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/100-copy-init-scalar-constructor-deleted-copy-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/100-direct-init-constructor-deleted-copy.t` | Retain boundary | N3485 8.5, 13.3.1.4: select the conversion, then validate final construction. |
| `pa12/tests/general/100-empty-member-constructor-parameter-lifetime.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/200-canonical-class-result-boundaries.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/200-conditional-class-const-result-category.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/200-conditional-class-glvalue-copy-initialization.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/200-conditional-class-glvalue-deleted-copy-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/200-constructor-argument-temporary-dtor.t` | Retain boundary | VALIDATION: compile-pass  N3485 focus: 12.2 [class.temporary], 12.6.2 [class.base.init] |
| `pa12/tests/general/200-reference-bound-constructor-temporary-backing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/200-reference-cv-base-const-volatile-xvalue-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/200-reference-cv-base-mutable-xvalue-cast-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/200-reference-cv-base-rvalue-reference-lvalue-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/200-reference-cv-base-rvalue-reference-xvalue.t` | Retain boundary | C++11 reference initialization: N3485 8.5.3. |
| `pa12/tests/general/200-reference-cv-class-const-volatile-rvalue-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/200-reference-cv-class-const-volatile-xvalue-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/200-reference-cv-class-volatile-rvalue-reference.t` | Retain boundary | C++11 reference initialization: N3485 8.5.3. |
| `pa12/tests/general/300-class-new-expression-default-constructor.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/300-class-specific-new-delete-selection.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/300-delete-class-pointer-lifetime.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa12/tests/general/300-move-only-aggregate-brace-member.t` | Retain boundary | A move-only aggregate member can be initialized from a class prvalue.  Construction may reuse the final member address under the copy-elision rules. |
| `pa12/tests/general/400-aggregate-member-conversion-ambiguous-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-ambiguous-constructor-group-conversion-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-ambiguous-function-group-constructor-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-argument-constructor-function-ambiguous-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-conditional-branch-destructor-demand.t` | Retain boundary | Both emitted arms require concrete destructor definitions, even when a  local condition selects only one of them at runtime. |
| `pa12/tests/general/400-conditional-class-bidirectional-conversion-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-conditional-class-explicit-conversion-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-conditional-class-value-conversion-fallback.t` | Retain boundary | A failed Tracker-to-Box& match must still consider Tracker-to-Box.  A reverse value conversion through Derived(Box const&) cannot compete  with this direct derived-to-base reference match. |
| `pa12/tests/general/400-copy-init-constructor-function-ambiguous-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-copy-init-source-cv-and-inherited-ranking.t` | Retain boundary | Independent namespaces check selection without sharing candidate inventories. |
| `pa12/tests/general/400-copy-init-user-conversion-chain-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-defaulted-constructor-function-ambiguous-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-direct-and-list-conversion-candidates.t` | Retain boundary | Independent namespaces check selection without sharing candidate inventories. |
| `pa12/tests/general/400-direct-list-init-user-conversion.t` | Retain boundary | One conversion of a constructor argument is valid outside class copy initialization. |
| `pa12/tests/general/400-explicit-conversion-hides-base-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-new-array-private-destructor-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-new-array-protected-base-destructor-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-new-private-destructor-access-contexts.t` | Retain boundary | Scalar allocation does not require destructor access; array allocation does. |
| `pa12/tests/general/400-private-preferred-class-conversion-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-ref-qualified-function-constructor-ambiguous-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-return-constructor-function-ambiguous-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-unqualified-function-rvalue-constructor-ambiguous-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa12/tests/general/400-zero-length-new-array.t` | Retain boundary | A zero bound in new[] is allowed; no element is constructed or destroyed. |
| `pa12/tests/spec/400-complete-class-special-member-exception-specification.t` | Retain boundary | Constructor names do not hide the injected class name in sizeof(type). |
| `pa13/tests/general/400-explicit-virtual-destructor-call-dispatch.t` | Retain boundary | N3485 [class.dtor]: explicit unqualified destructor calls can dispatch virtually. |
| `pa13/tests/general/400-explicit-virtual-destructor-call-nonvirtual.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa13/tests/general/400-inline-polymorphic-constructor-vtable.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa13/tests/spec/100-virtual-exception-rvalue-reference-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/200-complete-class-virtual-exception-specification-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/200-complete-class-virtual-exception-specification.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa13/tests/spec/200-virtual-exception-intermediate-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/200-virtual-exception-nonthrowing-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/200-virtual-exception-pointer-cv-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/200-virtual-exception-pointer-reference-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/200-virtual-exception-private-base-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/200-virtual-exception-unrestricted-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/200-virtual-exception-wrong-type-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/300-virtual-exception-implicit-destructor-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/300-virtual-exception-implicit-finite-union-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa13/tests/spec/300-virtual-exception-specification-subsets.t` | Retain boundary | VALIDATION: compile-pass  N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.  Declaration restrictions only; no throwing function body or EH control. |
| `pa14/tests/general/100-dependent-virtual-exception-subsets.t` | Retain boundary | VALIDATION: compile-pass  N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.  Declaration restrictions only; no throwing function body or EH control. |
| `pa14/tests/general/100-dependent-virtual-exception-wide-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/general/100-forward-declared-character-trait-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/general/100-forward-template-typedef-no-output-shell.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa14/tests/general/100-inherited-constructor-using-alias-template.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa14/tests/general/100-new-expression-completes-template-layout.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa14/tests/general/100-template-auto-trailing-return.t` | Retain boundary | HHC-090 |
| `pa14/tests/general/100-template-dependent-result-expression-identity.t` | Retain boundary | Sharing a dependent lookup root does not make different expressions equivalent. |
| `pa14/tests/general/100-template-dependent-result-parameter-position.t` | Retain boundary | Renaming preserves parameter identity; exchanging positions does not. |
| `pa14/tests/general/100-template-dependent-result-parameter-renaming.t` | Retain boundary | Function parameter names in result expressions compare by position. |
| `pa14/tests/general/100-template-fixed-result-duplicate-definition-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/general/100-template-fixed-result-lookup-ambiguous.t` | Retain boundary | Nondependent result types are part of the template signature. |
| `pa14/tests/general/100-template-fixed-result-lookup-merge.t` | Retain boundary | Different nondependent expressions can name the same result type. |
| `pa14/tests/general/100-template-fixed-result-spelling-merge.t` | Retain boundary | A fixed trailing decltype and an ordinary result spell the same type. |
| `pa14/tests/general/300-custom-character-trait-template-conversions.t` | Retain boundary | VALIDATION: compile-pass  Hosted character-trait reducer with a complete primary definition.  User character conversions are instantiated through ordinary template facts. |
| `pa14/tests/general/300-dependent-alias-class-result-function-pointer.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa14/tests/general/300-dependent-local-alias-new-constructor.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa14/tests/general/300-forward-template-typedef-no-output-shell.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa14/tests/general/300-inherited-constructor-using-alias-template.t` | Retain boundary | N3485 12.9: the terminal T names a constructor, not a new member. |
| `pa14/tests/spec/100-adjusted-parameter-sizeof-bound.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa14/tests/spec/100-template-fixed-expression-types.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa14/tests/spec/100-unused-template-sizeof-addition-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/100-unused-template-sizeof-argument-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/100-unused-template-sizeof-assignment-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/100-unused-template-sizeof-indirection-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/100-unused-template-sizeof-logical-bound-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/100-unused-template-sizeof-member-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/100-unused-template-sizeof-subscript-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-renamed-template-static-pointer-definition.t` | Retain boundary | VALIDATION: compile-pass  Static-member declaration legality is checked independently of storage demand. |
| `pa14/tests/spec/300-static-member-selective-demand.t` | Retain boundary | N3485 focus: 14.7.1 [temp.inst]/2,8,10: value and address use demand individual definitions. |
| `pa14/tests/spec/300-template-static-declarator-list-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-template-static-function-pointer-definition.t` | Retain boundary | VALIDATION: compile-pass  Static-member declaration legality is checked independently of storage demand. |
| `pa14/tests/spec/300-template-static-member-alias-definition.t` | Retain boundary | VALIDATION: compile-pass  Static-member declaration legality is checked independently of storage demand. |
| `pa14/tests/spec/300-unused-nested-template-static-duplicate-definition-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-unused-static-member-initializer.t` | Retain boundary | N3485 focus: 14.7.1 [temp.inst]/1,2,8,10: class use does not demand static definitions. |
| `pa14/tests/spec/300-unused-template-nonstatic-definition-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-unused-template-static-definition-type-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-unused-template-static-duplicate-definition-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-unused-template-static-function-pointer-type-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-unused-template-static-member-alias-type-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-unused-template-static-qualified-alias-type-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa14/tests/spec/300-unused-template-static-undeclared-member-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa15/tests/general/100-aggregate-array-final-element-address.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa15/tests/general/100-forward-declared-trait-value-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa15/tests/general/100-static-assert-integer-message-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa15/tests/general/100-static-assert-user-defined-message-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa15/tests/general/200-empty-pack-unknown-bound-array-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa15/tests/general/200-nested-call-pack-expansion-independent-packs.t` | Retain boundary | Keep both pack partitions observable in an emitted address, including at O2. |
| `pa15/tests/general/200-pack-array-bound-completion.t` | Retain boundary | Empty packs remain valid when a known bound or another element exists. |
| `pa15/tests/spec/100-renamed-template-static-dependent-array-definition.t` | Retain boundary | VALIDATION: compile-pass  Static-member declaration legality is checked independently of storage demand. |
| `pa15/tests/spec/300-static-specialization-declaration-before-definition.t` | Retain boundary | VALIDATION: compile-pass  Static-member declaration legality is checked independently of storage demand. |
| `pa15/tests/spec/300-static-specialization-duplicate-definition-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa16/tests/general/200-constexpr-noexcept-template-cache-default.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa16/tests/general/200-known-array-constexpr-call-storage.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa16/tests/general/200-pseudo-destructor-receiver-noexcept.t` | Retain boundary | N3485 [expr.pseudo]/1 evaluates the postfix-expression before dot/arrow.  [expr.unary.noexcept]/3 therefore includes its potentially throwing calls. |
| `pa16/tests/general/200-trivial-destructor-call-noexcept.t` | Retain boundary | Triviality does not override an explicit destructor exception specification. |
| `pa16/tests/general/300-constant-object-before-dynamic-startup.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa16/tests/general/300-constexpr-aggregate-array.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa16/tests/general/300-local-static-reference-before-dynamic-startup.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa16/tests/general/400-constexpr-reference-static-object-alias.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa17/tests/general/300-constant-template-object-before-dynamic-startup.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa17/tests/general/300-member-template-complete-class-exception-specification.t` | Retain boundary | This dependent specification is checked only when the member is needed. |
| `pa17/tests/general/300-member-template-nonconstant-exception-specification-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa17/tests/spec/200-primary-and-partial-static-definition-owners.t` | Retain boundary | VALIDATION: compile-pass  Static-member declaration legality is checked independently of storage demand. |
| `pa17/tests/spec/200-unused-partial-static-definition-type-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa17/tests/spec/300-explicit-class-static-initializer-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa17/tests/spec/300-explicit-class-static-storage-demand.t` | Retain boundary | N3485 focus: 14.7.2 [temp.explicit]/8,9: instantiate members defined at the class instantiation. |
| `pa17/tests/spec/300-explicit-static-member-initializer-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa17/tests/spec/300-nested-static-member-selective-demand.t` | Retain boundary | N3485 focus: 14.7.1 [temp.inst]/1,2,8,10: retain nested member definitions separately. |
| `pa17/tests/spec/300-specialized-static-member-address-demand.t` | Retain boundary | N3485 focus: 14.7.3 [temp.expl.spec]/6: a member specialization owns its definition. |
| `pa18/tests/general/300-copy-init-ambiguous-conversion-substitution.t` | Retain boundary | N3485 8.5, 13.3.1.4: select the conversion, then validate final construction. |
| `pa18/tests/general/300-copy-init-deleted-copy-substitution.t` | Retain boundary | N3485 8.5, 13.3.1.4: select the conversion, then validate final construction. |
| `pa18/tests/general/300-copy-init-template-and-ordinary-ranking.t` | Retain boundary | Independent namespaces check selection without sharing candidate inventories. |
| `pa18/tests/general/300-copy-init-two-conversion-templates-ambiguous-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa18/tests/general/300-dependent-enable-if-return-nontype-less-pack.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa18/tests/general/300-detected-or-alias-template-argument.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa18/tests/general/300-explicit-conversion-template-call-target-types.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa18/tests/general/300-explicit-template-call-transitive-base-deduction.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa18/tests/general/300-implicit-conversion-template-class-copy-init.t` | Retain boundary | A conversion function template produces the destination class directly. |
| `pa18/tests/general/300-private-type-class-instantiation-hard-error-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa18/tests/general/300-qualified-template-result-first-lookup.t` | Retain boundary | N3485 focus: 14.5.6.1 [temp.over.link] and 14.6.3 [temp.nondep].  A function-template result retains the first declaration's lookup even when  a later declaration changes the enclosing lookup set. |
| `pa19/tests/general/300-deleted-return-sfinae-same-parameter-list.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa19/tests/general/400-concrete-recursive-node-layout-retry.t` | Retain boundary | A concrete node specialization can be structurally completed while its  recursively referenced value type still lacks layout.  Once the value type  completes, a later node-allocation demand must rematerialize that same |
| `pa19/tests/spec/200-defaulted-class-template-argument-pack-prefix-deduction.t` | Retain boundary | VALIDATION: compile-pass  N3485 focus: 14.8.2.5 [temp.deduct.type] |
| `pa19/tests/spec/200-defaulted-complete-pack-inconsistent-deduction-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa19/tests/spec/200-defaulted-type-arguments-complete-deduced-pack.t` | Retain boundary | N3485 [temp.arg]/4 and [temp.deduct.type]/9: all defaults belong to the actual type.  Taking the address also retains the written box<T,Ts...> ABI pattern. |
| `pa19/tests/spec/200-defaulted-value-arguments-complete-deduced-pack.t` | Retain boundary | Completed integral defaults participate in the same pack deduction as explicit values.  Retain the value expansion in the function name as well as its concrete values. |
| `pa19/tests/spec/200-nonterminal-template-pack-explicit-and-separate-deduction.t` | Retain boundary | A written suffix makes the whole list non-deduced; other arguments can supply the pack. |
| `pa19/tests/spec/200-nonterminal-template-pack-nondeduction-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa20/tests/general/100-auto-const-reference-class-aggregate.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-class-borrowed-call.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-class-call.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-class-conditional.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-class-const-rvalue-auto.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-class-runtime.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-global-reference-runtime.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-mutable-rvalue-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa20/tests/general/100-auto-const-reference-scalar-array-reference.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-scalar-function-reference.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-scalar-runtime.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-scalar-rvalue.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/100-auto-const-reference-static-reference-runtime.t` | Combine | All successful boundaries retained in `pa20/tests/general/100-auto-const-reference-boundaries.t`; common harness factored where applicable. |
| `pa20/tests/general/200-captureless-lambda-copy-init-chain-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa20/tests/general/200-captureless-lambda-explicit-wrapper-construction.t` | Retain boundary | Constructors may convert their arguments during direct or list initialization. |
| `pa20/tests/general/200-captureless-lambda-return-chain-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa20/tests/general/200-default-argument-lambda-this-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa20/tests/general/200-lambda-constructor-deduction-preserves-closure.t` | Retain boundary | Constructor deduction uses the closure; conversion to a function pointer is later. |
| `pa20/tests/general/200-lambda-pointer-constructor-template-deduction-reject.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa20/tests/general/200-lambda-pointer-conversion-noexcept.t` | Retain boundary | CWG 1722: conversion to a function pointer does not invoke the lambda body. |
| `pa20/tests/general/200-unevaluated-lambda-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa21/tests/general/100-local-class-lambda-enclosing-capture-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa21/tests/general/100-source-nested-catch-ancestor-dispatch.t` | Retain boundary | N3485 [except.handle]: a miss searches dynamically enclosing handlers. |
| `pa21/tests/general/100-source-rethrow-through-function.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/100-typeid-template-template-argument-typeinfo-name.t` | Retain boundary | An exported accessor makes the same typeinfo observable to host object checks. |
| `pa21/tests/general/200-active-handler-conditional-temporary-lifetime.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-active-handler-full-expression-boundaries.t` | Retain boundary | N3485 [class.temporary], [except.handle], [except.throw]: temporaries  finish before the active handler releases its caught exception object. |
| `pa21/tests/general/200-aggregate-prefix-conditional-aggregate-temporaries.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-aggregate-prefix-constructor-member-array.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-aggregate-construction-prefix-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-aggregate-prefix-flat-members.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-aggregate-construction-prefix-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-aggregate-prefix-long-conditional-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-aggregate-construction-prefix-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-aggregate-prefix-matrix-member.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-aggregate-construction-prefix-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-aggregate-prefix-nested-member-array.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-aggregate-construction-prefix-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-aggregate-prefix-short-circuit-always-true.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-aggregate-prefix-template-aggregate-owner.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-aggregate-construction-prefix-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-aggregate-prefix-temporary-and-final-scalar.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-aggregate-construction-prefix-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-array-destructor-remaining-elements.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-auto-const-reference-auto-list-brace-owned-lifetime.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-auto-const-reference-auto-list-brace.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-auto-const-reference-auto-list-owned-lifetime.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-auto-const-reference-auto-list-volatile-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa21/tests/general/200-builtin-initializer-list-private-ctor-argument.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-conditional-aggregate-nothrow-copy-left.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-conditional-aggregate-nothrow-copy-right.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-conditional-arm-temporary-unwind-isolation.t` | Retain boundary | Keep a local literal condition to exercise the former suppression path. |
| `pa21/tests/general/200-conditional-constructor-left-throw.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-conditional-constructor-right-throw.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-conditional-scalar-local-right-throw.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-conditional-void-local-both-throw.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-borrowed-reference-does-not-extend-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-const-reference-cast-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-const-reference-conditional-unwind-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-const-reference-copy-list-noexcept.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-const-reference-direct-list-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-const-reference-empty-noexcept.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-const-reference-functional-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-const-reference-loop-prefix-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-copy-list-wrapper-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-global-reference.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-mutable-reference-negative.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa21/tests/general/200-initializer-list-returned-wrapper-does-not-extend-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-rvalue-reference-list-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-scalar-reference.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-static-reference.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-initializer-list-value-list-unwind-throwing.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-inner-handler-array-three.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-base-custom.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-delegating-rethrow.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-destructor-miss.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-destructor-nine-members.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-function-try-body-local.t` | Retain boundary | Destruction traces agree with independent Clang and GCC runs at O0/O2. |
| `pa21/tests/general/200-inner-handler-member-body-handler-locals.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-member-miss.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-member-replace.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-nested-outer-miss.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-nested-outer-swallow.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-inner-handler-ordinary-throwing-local.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-inner-handler-construction-boundaries.t`; common harness factored where applicable. |
| `pa21/tests/general/200-nonthrowing-try-fallthrough.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa21/tests/general/200-reference-initializer-aggregate-conditional-extended.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-aggregate-member-extended.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-call-does-not-extend.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-call-temporary-argument.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-cast-conditional-temporary.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-conditional-glvalue-two-temporaries.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-constructor-reference.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-destructor-reference.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-lvalue-right.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-nested-comma-extended.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-reference-call-right.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-scalar-const-reference-call.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-reference-initializer-xvalue-left.t` | Retain boundary | Destruction traces agree with independent Clang and GCC at O0/O2. |
| `pa21/tests/general/200-scalar-new-constructor-failure-deallocation.t` | Retain boundary | Failed construction destroys the argument temporary and releases storage.  C++11 permits evaluating arguments before null allocation; destroy only  arguments that were constructed and never call the item constructor. |
| `pa21/tests/general/200-source-exception-empty-scope-boundary.t` | Retain boundary | N3485 [except.ctor]: an empty lexical scope still bounds unwinding. |
| `pa21/tests/general/200-source-imported-lexical-destructor-tail.t` | Retain boundary | Imported destructors can throw even when this unit contains no throw/catch. |
| `pa21/tests/general/200-source-lexical-body-unwind-tail.t` | Retain boundary | N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.  function-try-local-return  function-try-constructor-local |
| `pa21/tests/general/200-source-lexical-catch-parameter-order.t` | Retain boundary | N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.  named-catch-fallthrough  catch-parameter-throws-named |
| `pa21/tests/general/200-source-lexical-handler-return-order.t` | Retain boundary | N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.  return-value  return-void |
| `pa21/tests/general/200-source-lexical-handler-scope-order.t` | Retain boundary | N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.  normal-handler-end  nested-block-end |
| `pa21/tests/general/200-source-lexical-return-unwind-tail.t` | Retain boundary | N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.  ordinary-return-tail  ordinary-scope-tail |
| `pa21/tests/general/200-source-nested-catch-prefix-order.t` | Retain boundary | N3485 [except.ctor], [except.handle]: destroy each intervening prefix once. |
| `pa21/tests/general/200-source-nested-handler-lifetime-forwarding.t` | Retain boundary | N3485 [except.handle], [except.throw]: locals finish before active catches. |
| `pa21/tests/general/200-source-nested-handler-temporary-forwarding.t` | Retain boundary | N3485 [class.temporary], [except.handle]: destroy arguments before catches. |
| `pa21/tests/general/200-source-unnamed-catch-value-forwarding.t` | Retain boundary | N3485 [except.handle]: unnamed catch parameters have their own lifetime. |
| `pa21/tests/general/200-synthesized-copy-array-3-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-array-after-member-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-array-only-3-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-base-custom-destructor-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-defaulted-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-empty-base-prefix-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-flat-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-member-custom-destructor-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-trivial-copy-array-prefix-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-trivial-copy-base-prefix-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-trivial-copy-member-prefix-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-copy-trivial-copy-union-prefix-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-move-array-12-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-synthesized-move-flat-construction-prefix.t` | Combine | All successful boundaries retained in `pa21/tests/general/200-synthesized-copy-move-construction-prefix.t`; common harness factored where applicable. |
| `pa21/tests/general/200-terminated-conditional-arm-cleanup-state.t` | Retain boundary | A throw terminates one arm. Its cleanup region and temporary state must  not become the starting state of the sibling or nested conditional. |
| `pa21/tests/general/300-pseudo-destructor-throwing-receiver.t` | Retain boundary | Receiver evaluation remains part of the enclosing full expression. |
| `pa22/tests/general/100-base-same-type-alias-convergence.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa22/tests/general/100-public-qualified-base-typedef-ambiguous-subobject.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa22/tests/general/100-static-base-subobject-relocations.t` | Retain boundary | Preserve the binding identity and the member/array origin. |
| `pa22/tests/general/100-static-nonprimary-base-relocations.t` | Retain boundary | Static addresses do not require constant object values. |
| `pa22/tests/general/300-member-function-pointer-contextual-bool.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa22/tests/general/300-member-function-template-pointer-target-init.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa22/tests/general/300-member-pointer-base-to-derived-data-access.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa22/tests/general/300-member-pointer-base-to-derived-function-access.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa23/tests/general/100-constructor-prvalue-virtual-base-forwarding.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa23/tests/general/100-inverse-member-pointer-virtual-dispatch-and-constraints.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa23/tests/general/100-shared-virtual-exception-subsets.t` | Retain boundary | VALIDATION: compile-pass  N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.  Declaration restrictions only; no throwing function body or EH control. |
| `pa23/tests/general/100-shared-virtual-exception-wide-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa23/tests/general/100-sibling-dynamic-cast.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa23/tests/general/100-static-member-virtual-base-relocations.t` | Retain boundary | An unproved member's complete layout must retain runtime projection. |
| `pa23/tests/general/100-static-virtual-base-relocations.t` | Retain boundary | A known complete object supplies its virtual-base layout. |
| `pa23/tests/general/100-virtual-base-constructor-vptr-hidden-target.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa23/tests/general/100-virtual-base-pointer-null-adjustment.t` | Retain boundary | Test null preservation before reading the virtual-base offset. |
| `pa23/tests/general/100-virtual-inheritance-fields.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa24/tests/strict/100-startup-shutdown-hooks.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa27/tests/general/200-host-nontrivial-sixteen-byte-class-result.t` | Retain boundary | Host-object ABI: nontrivial 16-byte results use caller storage in both directions. |
| `pa28/tests/general/200-host-array-destructor-second-fault-terminate.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa28/tests/general/200-host-eh-unwind-destructor-terminate.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa28/tests/general/200-host-lexical-double-fault-termination.t` | Retain boundary | Driver for 200-host-lexical-double-fault-termination. |
| `pa29/tests/compile/500-builtin-trivial-declaration-properties.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa29/tests/compile/500-builtin-trivial-deleted-copy.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa29/tests/compile/500-builtin-trivial-shared-subobjects.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa29/tests/compile/500-conditional-explicit-specifier.t` | Retain boundary | A conditional explicit-specifier decides whether the constructor takes part  in copy initialization, and an attribute may sit between it and the rest of  the declaration, which is how libc++ writes pair's constructors. |
| `pa29/tests/compile/500-gnu-attribute-declarations.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa29/tests/compile/500-standard-noreturn-arguments-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa29/tests/compile/500-standard-noreturn-variable-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa29/tests/compile/600-builtin-nothrow-user-provided-constructor.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa29/tests/compile/600-hosted-nothrow-default-constructible-shorthand.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa29/tests/compile/600-inherited-constructor-excludes-base-special-members.t` | Retain boundary | N3485 12.9: the other subobjects must be default-initializable. |
| `pa29/tests/compile/700-gnu-alignof-qualified-member-expression.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa29/tests/compile/700-hosted-char-traits-primary-conversion-shims.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa29/tests/compile/700-hosted-nothrow-invocable-cache-default.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa29/tests/compile/700-hosted-trait-template-bodies.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa30/tests/compile/400-base-same-type-alias-ambiguous-bad.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa30/tests/compile/400-reachable-missing-return-bad.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa30/tests/compile/700-hosted-replaceable-operator-new-dynamic-exception-spec.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa30/tests/compile/700-hosted-replaceable-operator-new-wrong-dynamic-exception-spec-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa30/tests/compile/700-hosted-replaceable-operator-new.t` | Retain boundary | VALIDATION: compile-pass  A C++11 replacement matches the unrestricted declaration in <new>. |
| `pa32/tests/o0/100-bad-comparison-width.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa33/tests/driver/100-alignas-class-layout.t` | Retain route | Same source body; native optimized/object/link routes add backend coverage. |
| `pa33/tests/driver/200-nonthrowing-try-fallthrough.t` | Retain route | Same source body; native optimized/object/link routes add backend coverage. |
| `pa33/tests/driver/200-source-nested-handler-lifetime-forwarding.t` | Retain route | Same source body; native optimized/object/link routes add backend coverage. |
| `pa33/tests/driver/200-synthesized-copy-trivial-copy-array-prefix-construction-prefix.t` | Combine route | Same source body; native optimized/object/link routes add backend coverage. |
| `pa33/tests/driver/200-synthesized-move-array-12-construction-prefix.t` | Combine route | Same source body; native optimized/object/link routes add backend coverage. |
| `pa33/tests/driver/300-defined-nonvoid-control-flow.t` | Retain route | Same source body; native optimized/object/link routes add backend coverage. |
| `pa33/tests/driver/400-sibling-dynamic-cast.t` | Retain route | Same source body; native optimized/object/link routes add backend coverage. |
| `pa33/tests/driver/500-conditional-explicit-specifier.t` | Retain route | Same source body; native optimized/object/link routes add backend coverage. |
| `pa33/tests/o1/200-disjoint-object-slots.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa6/tests/general/200-adjusted-parameter-sizeof-bound.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa6/tests/general/200-namespace-type-convergence.t` | Retain boundary | AUDIT-ID: LOOKUP-NAMESPACE  AUDIT-EXPECT: compile  N3485 7.1.3 and 7.3.4: aliases of the same type converge through |
| `pa6/tests/general/300-ambiguous-using-directive-type-bad.t` | Retired previously | Prior qualified replacement/removal recorded in the unified tracker. |
| `pa6/tests/general/300-complete-class-exception-specification.t` | Retain boundary | CWG 1330: member exception specifications are complete-class contexts. |
| `pa6/tests/general/300-distinct-using-directive-types-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa6/tests/general/300-dynamic-exception-specification-different-set-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa6/tests/general/300-dynamic-exception-specification-set.t` | Retain boundary | AUDIT-ID: EH-SPEC-SET  AUDIT-EXPECT: compile  N3485 15.4/3: order and duplicates do not change the allowed type set. |
| `pa6/tests/general/300-namespace-typedef-value-ambiguity-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa6/tests/general/300-unused-nonconstant-exception-specification-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa6/tests/general/300-unused-unknown-exception-name-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa7/tests/general/300-distinct-using-directive-values-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa7/tests/general/300-namespace-type-value-ambiguity-bad.t` | Retain rejection | Independent invalid declaration/expression must be rejected; combining it with another rejection would mask the later error. |
| `pa9/tests/abi/100-constructor-destructor-variants.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa9/tests/abi/100-namespace-variable.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa9/tests/abi/300-template-template-argument.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa9/tests/abi/500-dependent-function-parameter-decltype-param.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |
| `pa9/tests/abi/600-template-param-template-type-substitution.t` | Retain boundary | Distinct observable language/property boundary; see its family in the unified tracker. |

The additional PA28 return termination companion extends an existing driver. Its old ordinary double-fault property remains covered by the lexical-termination neighbor. The two optimized array audit aliases become one combined alias. The original standalone handler control points at the combined source; its pre-existing serialized RTTI failure is fixed and independently qualified in a34bf7b16. Final export checks the two student-tool route, with no additional owning source body.

## Companion and alias inputs

| Input | Disposition | Purpose |
| --- | --- | --- |
| `pa25/tests/general/300-runtime-destructor-function-try-block.t.1` | Retain paired input | Companion/provider belongs to its host-runtime driver; preserves an ABI or runtime boundary. |
| `pa27/tests/general/200-host-nontrivial-sixteen-byte-class-result.lib.provider.cpp` | Retain paired input | Companion/provider belongs to its host-runtime driver; preserves an ABI or runtime boundary. |
| `pa27/tests/general/200-host-nontrivial-sixteen-byte-class-result.t.1` | Retain paired input | Companion/provider belongs to its host-runtime driver; preserves an ABI or runtime boundary. |
| `pa28/tests/controls/100-audit-standalone-function-try.audit` | Retain paired input | Opt-in route to its recorded source; no additional fixture body. |
| `pa28/tests/controls/100-audit-standalone-handler-forwarding.audit` | Retain paired input | Opt-in route to its recorded source; no additional fixture body. |
| `pa28/tests/general/200-host-array-destructor-second-fault-terminate.t.1` | Retain paired input | Companion/provider belongs to its host-runtime driver; preserves an ABI or runtime boundary. |
| `pa28/tests/general/200-host-eh-unwind-destructor-terminate.t.1` | Retain paired input | Companion/provider belongs to its host-runtime driver; preserves an ABI or runtime boundary. |
| `pa28/tests/general/200-host-lexical-double-fault-termination.t.1` | Retain paired input | Companion/provider belongs to its host-runtime driver; preserves an ABI or runtime boundary. |
| `pa29/tests/run/800-builtin-assume-aligned-two-arg.t.1` | Retain paired input | Companion/provider belongs to its host-runtime driver; preserves an ABI or runtime boundary. |
| `pa32/tests/controls/500-audit-backward-loop-residues.audit` | Retain paired input | Opt-in route to its recorded source; no additional fixture body. |
| `pa33/tests/controls/200-audit-optimized-copy-move.audit` | Retain paired input | Opt-in route to its recorded source; no additional fixture body. |

The new serialized LowIR route in PA33 reuses every driver source through the two student tools; no extra owning source is added for the runtime alias repair.
