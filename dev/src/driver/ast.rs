// Translated from cppgm++.cpp::run_emit_ast_mode. See NOTICE.
//! One source owner and one parsed syntax unit at a time.
use crate::preprocess::{error::PreprocessError, options::PredefinedValues, unit::Preprocessor};
use crate::support::source::SourceBuffer;
use crate::syntax::{ParseError, SyntaxUnit};
#[derive(Debug)]
pub enum AstToolError {
    Initialization(PreprocessError),
    Diagnostic(Vec<u8>),
    Missing(&'static str),
    Internal(&'static str),
    Output(std::io::Error),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EmitMode {
    Ast,
    Types,
    Semantics,
    LowIr,
}
#[derive(Default)]
pub struct LowIrOutput {
    output: crate::lower::Output,
}
impl LowIrOutput {
    /// Render the typed program accumulated across translation units.
    ///
    /// # Errors
    /// Returns an output failure.
    pub fn write(&self, output: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        crate::lowir::writer::write_globals(
            &self.output.program,
            output,
            !self.output.definitions.is_empty() || self.output.initializer.is_some(),
        )?;
        output.write_all(&self.output.definitions)?;
        if let Some(symbol) = self.output.initializer {
            let function = self
                .output
                .program
                .function(self.output.program.symbol(symbol))
                .ok_or_else(|| std::io::Error::other("missing startup function"))?;
            crate::lowir::writer::write_definition(&self.output.program, function, output)?;
        }
        crate::lowir::writer::write_aliases(&self.output.program, output)?;
        Ok(())
    }
}
pub enum Destination<'a> {
    Text(&'a mut dyn std::io::Write),
    LowIr(&'a mut LowIrOutput),
}
/// Parse and print one translation unit using the shared typed front end.
///
/// # Errors
/// Returns a located input failure, missing group, or output error.
pub fn emit_unit(
    source: &SourceBuffer,
    path: &[u8],
    values: &PredefinedValues,
    destination: Destination<'_>,
    ordinal: usize,
    stack_budget: crate::support::stack::StackBudget,
    mode: EmitMode,
) -> Result<(), AstToolError> {
    let preprocessor = Preprocessor::with_options(
        source,
        path,
        crate::preprocess::files::FileServices {
            identity: super::file_identity::query,
            read: super::file_identity::read,
        },
        values,
    )
    .map_err(AstToolError::Initialization)?;
    let mut unit = SyntaxUnit::new(preprocessor);
    unit.set_stack_budget(stack_budget);
    if let Err(error) = unit.parse() {
        return Err(match error {
            ParseError::NotImplemented(group)
            | ParseError::Preprocess(PreprocessError::NotImplemented(group)) => {
                AstToolError::Missing(group)
            }
            ParseError::Invariant(message)
            | ParseError::Preprocess(PreprocessError::Invariant(message)) => {
                AstToolError::Internal(message)
            }
            _ => AstToolError::Diagnostic(
                unit.describe(&error)
                    .map_err(AstToolError::Initialization)?,
            ),
        });
    }
    let graph = if mode == EmitMode::Ast {
        None
    } else {
        let (tree, cursor, declarations) = unit
            .semantic_inputs()
            .map_err(|_| AstToolError::Internal("missing syntax"))?;
        let analysis =
            crate::sema::Analyzer::new(tree, cursor, stack_budget).and_then(|mut analyzer| {
                analyzer.set_semantics(matches!(mode, EmitMode::Semantics | EmitMode::LowIr));
                if mode == EmitMode::LowIr {
                    analyzer.set_lowering();
                }
                analyzer.analyze(declarations)
            });
        Some(analysis.map_err(|error| match error {
            crate::sema::Error::Diagnostic { location, message }
            | crate::sema::Error::NonConstant { location, message } => {
                match unit.describe(&ParseError::Syntax { location, message }) {
                    Ok(bytes) => AstToolError::Diagnostic(bytes),
                    Err(error) => AstToolError::Initialization(error),
                }
            }
            crate::sema::Error::Missing(group) => AstToolError::Missing(group),
            crate::sema::Error::Limit(message) => {
                AstToolError::Diagnostic(message.as_bytes().to_vec())
            }
            crate::sema::Error::Invariant(message) => AstToolError::Internal(message),
        })?)
    };
    let output = match destination {
        Destination::LowIr(output) => {
            let graph = graph.ok_or(AstToolError::Internal("lowering without semantic graph"))?;
            let (tree, cursor, declarations) = unit
                .semantic_inputs()
                .map_err(|_| AstToolError::Internal("missing syntax"))?;
            crate::lower::lower_unit(
                &mut output.output,
                &graph,
                tree,
                cursor,
                declarations,
                stack_budget,
            )
            .map_err(|error| match error {
                crate::lower::Error::Missing(group)
                | crate::lower::Error::Semantic(crate::sema::Error::Missing(group)) => {
                    AstToolError::Missing(group)
                }
                crate::lower::Error::Semantic(crate::sema::Error::Limit(message)) => {
                    match unit.describe(&ParseError::Syntax {
                        location: crate::support::source::SourceOffset(0),
                        message,
                    }) {
                        Ok(bytes) => AstToolError::Diagnostic(bytes),
                        Err(error) => AstToolError::Initialization(error),
                    }
                }
                crate::lower::Error::Semantic(
                    crate::sema::Error::Diagnostic { location, message }
                    | crate::sema::Error::NonConstant { location, message },
                ) => match unit.describe(&ParseError::Syntax { location, message }) {
                    Ok(bytes) => AstToolError::Diagnostic(bytes),
                    Err(error) => AstToolError::Initialization(error),
                },
                crate::lower::Error::Semantic(crate::sema::Error::Invariant(message)) => {
                    AstToolError::Internal(message)
                }
                crate::lower::Error::Abi(error) => match error {
                    crate::abi::Error::Diagnostic(bytes) => AstToolError::Diagnostic(bytes),
                    crate::abi::Error::NotImplemented(group) => AstToolError::Missing(group),
                    crate::abi::Error::StorageLimit => AstToolError::Internal("ABI storage limit"),
                },
                crate::lower::Error::Ir(error) => match error {
                    crate::lowir::Error::NotImplemented(group) => AstToolError::Missing(group),
                    crate::lowir::Error::Output(error) => AstToolError::Output(error),
                    crate::lowir::Error::Limit => AstToolError::Internal("LowIR storage limit"),
                    _ => AstToolError::Internal("typed LowIR construction failed"),
                },
            })?;
            return Ok(());
        }
        Destination::Text(output) => output,
    };
    writeln!(output, "start translation unit {ordinal}").map_err(AstToolError::Output)?;
    if let Some(graph) = graph {
        let (tree, cursor, declarations) = unit
            .semantic_inputs()
            .map_err(|_| AstToolError::Internal("missing syntax"))?;
        if mode == EmitMode::Semantics {
            graph
                .print_semantics(tree, cursor, declarations, output)
                .map_err(AstToolError::Output)?;
        } else {
            graph
                .print(tree, cursor, output)
                .map_err(AstToolError::Output)?;
        }
    } else {
        unit.print(output).map_err(AstToolError::Output)?;
    }
    writeln!(output, "end translation unit").map_err(AstToolError::Output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::stack::StackBudget;

    #[test]
    fn inherited_inline_bodies_coalesce_across_units_and_internal_types_stay_local() {
        for internal in [false, true] {
            let mut output = LowIrOutput::default();
            for (index, name) in ["f", "g"].into_iter().enumerate() {
                let classes = "struct B{int v;int read()const{return v;}};struct D:B{int w;};";
                let source = if internal {
                    format!("namespace{{{classes}}} int {name}(D& d){{return d.read();}}")
                } else {
                    format!("{classes} int {name}(D& d){{return d.read();}}")
                };
                let source = SourceBuffer::new(source.into_bytes()).unwrap();
                emit_unit(
                    &source,
                    b"inherited-unit-demand",
                    &PredefinedValues::default(),
                    Destination::LowIr(&mut output),
                    index + 1,
                    StackBudget::default(),
                    EmitMode::LowIr,
                )
                .unwrap();
            }
            let program = &output.output.program;
            let methods: Vec<_> = program
                .functions()
                .iter()
                .filter(|function| program.names.get(function.name).contains("B__read"))
                .collect();
            assert_eq!(methods.len(), if internal { 2 } else { 1 });
            for method in methods {
                assert!(method.body.is_none(), "streamed inline bodies are released");
                assert_eq!(
                    method.metadata.binding,
                    Some(if internal {
                        crate::lowir::vocabulary::Binding::Internal
                    } else {
                        crate::lowir::vocabulary::Binding::Weak
                    })
                );
                assert_eq!(
                    program.signature(method.signature).parameters[0].object_bytes,
                    4
                );
            }
            for function in program
                .functions()
                .iter()
                .filter(|function| matches!(program.names.get(function.name), "f" | "g"))
            {
                assert_eq!(
                    function.metadata.binding,
                    Some(if internal {
                        crate::lowir::vocabulary::Binding::Internal
                    } else {
                        crate::lowir::vocabulary::Binding::Strong
                    })
                );
                if internal {
                    let object = program.names.get(function.metadata.object.unwrap());
                    assert!(
                        object.starts_with("_Z1"),
                        "type-local signature does not give the name static linkage"
                    );
                }
            }
            let mut text = Vec::new();
            output.write(&mut text).unwrap();
            assert!(
                !std::str::from_utf8(&text)
                    .unwrap()
                    .contains("declare function @g(")
            );
        }
    }

    #[test]
    fn memory_builtin_boundaries_survive_frontends_without_leaking_into_indirect_types() {
        let mut output = LowIrOutput::default();
        for (index, source) in [
            b"inline void unused(char*d,const char*s){__builtin_memmove(d,s,4);}void copy(char*d,const char*s){__builtin_memcpy(d,s,4);}unsigned long length(const char*s){return __builtin_strlen(s);}".as_slice(),
            b"void again(char*d,const char*s){__builtin_memcpy(d,s,4);}void*(*p)(void*,const void*,unsigned long)=__builtin_memcpy;void*indirect(void*d,const void*s){return p(d,s,4);}".as_slice(),
        ].into_iter().enumerate() {
            let source = SourceBuffer::new(source.to_vec()).unwrap();
            emit_unit(&source, b"memory-boundaries", &PredefinedValues::default(),
                Destination::LowIr(&mut output), index + 1, StackBudget::default(), EmitMode::LowIr).unwrap();
        }
        let program = &output.output.program;
        let declarations: Vec<_> = program
            .functions()
            .iter()
            .filter(|f| program.names.get(f.name) == "__builtin_memcpy")
            .collect();
        assert_eq!(declarations.len(), 1, "cross-unit builtin symbols coalesce");
        let signature = program.signature(declarations[0].signature);
        assert!(signature.parameters[0].no_alias && signature.parameters[1].no_alias);
        assert!(
            signature
                .parameters
                .iter()
                .all(|p| p.pass.is_none() && p.object_bytes == 0)
        );
        assert!(matches!(
            signature.boundary.unwind,
            crate::lowir::model::Unwind::No
        ));
        assert_eq!(
            program.names.get(declarations[0].metadata.object.unwrap()),
            "cppgm_builtin_memcpy"
        );
        let mut text = Vec::new();
        output.write(&mut text).unwrap();
        let text = std::str::from_utf8(&text).unwrap();
        assert!(
            !text.contains("__builtin_memmove"),
            "unused inline bodies introduce no emission demand"
        );
        assert!(text.contains("effects=readonly"));
        let indirect = text.lines().find(|line| line.contains(" as (")).unwrap();
        assert!(
            !indirect.contains("alias=noalias") && !indirect.contains("unwind=no"),
            "entity facts must not enter a canonical pointer signature"
        );
    }

    #[test]
    fn integral_address_images_retain_their_storage_type_after_frontend_release() {
        // The frozen frontend emits this i64 address even though its text
        // reader accepts only ptr address items. Preserve that producer fact.
        let source = SourceBuffer::new(
            b"int x; unsigned long a[2]={(unsigned long)&x,0}; int main(){return 0;}".to_vec(),
        )
        .unwrap();
        let mut output = LowIrOutput::default();
        emit_unit(
            &source,
            b"address-images",
            &PredefinedValues::default(),
            Destination::LowIr(&mut output),
            1,
            StackBudget::default(),
            EmitMode::LowIr,
        )
        .unwrap();
        drop(source);
        let mut text = Vec::new();
        output.write(&mut text).unwrap();
        assert!(
            text.windows(b"i64 addr @x".len())
                .any(|s| s == b"i64 addr @x")
        );
        assert!(
            !text
                .windows(b"ptr addr @x".len())
                .any(|s| s == b"ptr addr @x")
        );
    }
    #[test]
    fn block_aliases_share_emission_identity_after_each_frontend_is_released() {
        let mut output = LowIrOutput::default();
        for (index, source) in [
            b"namespace N{inline int f(int x){return x;}int call(){int f(int);extern int value;return f(value);}}int main(){return N::call();}".as_slice(),
            b"namespace N{int value=0;int f(int);int other(){return f(value);}}".as_slice(),
        ].into_iter().enumerate() {
            let source = SourceBuffer::new(source.to_vec()).unwrap();
            emit_unit(&source, b"block-aliases", &PredefinedValues::default(),
                Destination::LowIr(&mut output), index + 1, StackBudget::default(), EmitMode::LowIr).unwrap();
        }
        // These symbols and signatures own only LowIR names/IDs. Both syntax
        // units and semantic graphs have already been dropped by emit_unit.
        let program = &output.output.program;
        for spelling in ["N__f", "N__value"] {
            assert_eq!(
                program
                    .symbols()
                    .iter()
                    .filter(|symbol| program.names.get(symbol.name) == spelling)
                    .count(),
                1
            );
        }
        let function = program
            .functions()
            .iter()
            .find(|function| program.names.get(function.name) == "N__f")
            .unwrap();
        assert!(function.body.is_none(), "streamed body must be released");
        assert_eq!(
            program
                .names
                .get(program.signature(function.signature).parameters[0].name),
            "x"
        );
        let mut text = Vec::new();
        output.write(&mut text).unwrap();
        let text = std::str::from_utf8(&text).unwrap();
        assert_eq!(text.matches("function @N__f(").count(), 1);
        assert!(
            !text.contains("slot $value"),
            "extern aliases own no frame storage"
        );
    }
}
