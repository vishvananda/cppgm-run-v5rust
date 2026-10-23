//! Filter the configured host's macros and include search. Translated from
//! `driver/host_environment.cpp`, `HostMacroText/add_host_environment`. See NOTICE.
use crate::preprocess::options::DriverOptions;

/// Configuration of the build-selected host, captured at the executable boundary.
pub struct HostEnvironment<'a> {
    pub predefined_macros: &'a [u8],
    pub include_paths: &'a [u8],
}

impl HostEnvironment<'_> {
    /// Add hosted predefined macros and the configured standard search chain.
    pub fn apply(&self, options: &mut DriverOptions, nostdinc: bool) {
        options
            .host_predefines
            .extend_from_slice(b"#define __STDC__ 1\n");
        for line in self.predefined_macros.split(|&b| b == b'\n') {
            let Some(definition) = line.strip_prefix(b"#define ") else {
                continue;
            };
            let end = definition
                .iter()
                .position(|&b| b == b' ' || b == b'(')
                .unwrap_or(definition.len());
            if import_macro(&definition[..end]) {
                options.host_predefines.extend_from_slice(line);
                options.host_predefines.push(b'\n');
            }
        }
        if !nostdinc {
            options.system_include_paths.extend(
                self.include_paths
                    .split(|&b| b == b'\n')
                    .filter(|p| !p.is_empty())
                    .map(<[u8]>::to_vec),
            );
        }
    }
}

fn import_macro(name: &[u8]) -> bool {
    if name.len() < 2 || name[0] != b'_' || !(name[1] == b'_' || name[1].is_ascii_uppercase()) {
        return false;
    }
    if matches!(
        name,
        b"__cplusplus"
            | b"__STDC_HOSTED__"
            | b"__DATE__"
            | b"__TIME__"
            | b"__FILE__"
            | b"__LINE__"
            | b"__CPPGM__"
            | b"__CPPGM_AUTHOR__"
            | b"__STDC__"
            | b"__SIZEOF_FLOAT128__"
            | b"__SIZEOF_FLOAT80__"
    ) {
        return false;
    }
    ![
        b"__BFLT16_".as_slice(),
        b"__FLT16_",
        b"__FLT32_",
        b"__FLT64_",
        b"__FLT128_",
        b"__FLT32X_",
        b"__FLT64X_",
        b"__FLT128X_",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_inputs_keep_search_order_and_only_supported_reserved_macros() {
        let mut options = DriverOptions::default();
        options.system_include_paths.push(b"explicit".to_vec());
        let host = HostEnvironment {
            predefined_macros: b"#define unix 1\n#define _lower 1\n#define __cplusplus 7\n#define __DATE__ bad\n#define __FLT128_MAX__ unsupported\n#define __SIZEOF_FLOAT80__ 16\n#define __GNUC__ 15\n#define __INTMAX_C(c) c ## L\n#define _TARGET 9\n",
            include_paths: b"first\nsecond\n",
        };
        host.apply(&mut options, false);
        assert_eq!(options.host_predefines, b"#define __STDC__ 1\n#define __GNUC__ 15\n#define __INTMAX_C(c) c ## L\n#define _TARGET 9\n");
        assert_eq!(
            options.system_include_paths,
            [b"explicit".to_vec(), b"first".to_vec(), b"second".to_vec()]
        );
        let mut isolated = DriverOptions::default();
        host.apply(&mut isolated, true);
        assert_eq!(isolated.host_predefines, options.host_predefines);
        assert!(isolated.system_include_paths.is_empty());
    }

    #[test]
    fn host_definitions_yield_once_and_builtin_queries_share_parenthesis_facts() {
        use crate::{
            preprocess::{files::FileServices, options::PredefinedValues, unit::Preprocessor},
            support::{source::SourceBuffer, stack::StackBudget},
            syntax::SyntaxUnit,
        };
        let host = HostEnvironment {
            predefined_macros: b"#define __GNUC__ 15\n",
            include_paths: b"",
        };
        let mut options = DriverOptions::default();
        host.apply(&mut options, false);
        for (source, accepts) in [
            (
                "#define __GNUC__ 4\n#if !__has_builtin(__builtin_is_constant_evaluated) || __has_builtin(__builtin_missing)\n#error query\n#endif\nstatic_assert(__GNUC__==4,\"override\");static_assert((__builtin_is_constant_evaluated()),\"evaluation\");static_assert(noexcept(__builtin_is_constant_evaluated()),\"noexcept\");bool f(){return (__builtin_is_constant_evaluated());}",
                true,
            ),
            ("#define __GNUC__ 4\n#define __GNUC__ 5\n", false),
        ] {
            let source = SourceBuffer::new(source.as_bytes().to_vec()).unwrap();
            let input = Preprocessor::with_driver_options(
                &source,
                b"host-input",
                FileServices {
                    identity: |_| None,
                    read: |_| None,
                },
                &PredefinedValues::default(),
                &options,
            )
            .unwrap();
            let mut unit = SyntaxUnit::new(input);
            if !accepts {
                assert!(
                    unit.parse().is_err(),
                    "ordinary source redefinitions stay strict"
                );
                continue;
            }
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let syntax_nodes = tree.expressions.mark();
            let _graph = crate::sema::analyze_unit(
                tree,
                cursor,
                declarations,
                StackBudget::default(),
                true,
                true,
            )
            .unwrap();
            assert_eq!(
                tree.expressions.mark(),
                syntax_nodes,
                "query analysis retains the parsed expression owners"
            );
        }
    }

    #[test]
    fn query_placeholder_is_a_type_fact_without_a_runtime_function_designation() {
        use crate::{
            preprocess::unit::Preprocessor,
            support::{source::SourceBuffer, stack::StackBudget},
            syntax::SyntaxUnit,
        };
        for semantic in [false, true] {
            let source =
                SourceBuffer::new(b"auto p=__builtin_is_constant_evaluated;".to_vec()).unwrap();
            let input = Preprocessor::with_name(&source, b"query-designation").unwrap();
            let mut unit = SyntaxUnit::new(input);
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let result = crate::sema::analyze_unit(
                tree,
                cursor,
                declarations,
                StackBudget::default(),
                semantic,
                false,
            );
            if semantic {
                assert!(matches!(
                    result,
                    Err(crate::sema::Error::Diagnostic {
                        message: "a builtin expanded where it is called named other than in a call",
                        ..
                    })
                ));
            } else {
                assert!(
                    result.is_ok(),
                    "the types view retains the placeholder signature"
                );
            }
        }
    }
}
