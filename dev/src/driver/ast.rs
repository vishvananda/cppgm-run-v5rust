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
    #[cfg(test)]
    pub(crate) fn for_object() -> Result<Self, crate::lowir::Error> {
        Self::for_object_model(crate::native::ObjectModel::Course)
    }
    pub(crate) fn for_object_model(
        model: crate::native::ObjectModel,
    ) -> Result<Self, crate::lowir::Error> {
        let mut output = crate::lower::Output::default();
        output
            .program
            .set_definition_sink(Box::new(crate::native::ObjectBuilder::with_model(
                0, model,
            )?))?;
        output.set_object_mode(model == crate::native::ObjectModel::Host);
        Ok(Self { output })
    }
    pub(crate) fn finish_object(
        mut self,
        path: Vec<u8>,
    ) -> Result<crate::native::Object, AstToolError> {
        self.output.finish_initialization().map_err(lower_error)?;
        self.output.finish_entry_aliases();
        crate::native::compile_object(&mut self.output.program, path, 0).map_err(native_error)
    }

    /// Render the typed program accumulated across translation units.
    ///
    /// # Errors
    /// Returns an output failure.
    pub fn write(&mut self, output: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        self.output.finish_initialization().map_err(|error| {
            std::io::Error::other(format!("startup finalization failed: {error:?}"))
        })?;
        self.output.finish_entry_aliases();
        crate::lowir::writer::write_globals(
            &self.output.program,
            output,
            !self.output.definitions.is_empty()
                || self.output.initializer.is_some()
                || self.output.finalizer.is_some(),
        )?;
        self.output
            .definitions
            .write(&self.output.program, output)?;
        for symbol in [self.output.initializer, self.output.finalizer]
            .into_iter()
            .flatten()
        {
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
    emit_preprocessed(preprocessor, destination, ordinal, stack_budget, mode)
}
pub(crate) fn emit_compilation_unit(
    source: &SourceBuffer,
    path: &[u8],
    values: &PredefinedValues,
    options: &crate::preprocess::options::DriverOptions,
    output: &mut LowIrOutput,
    budget: crate::support::stack::StackBudget,
) -> Result<(), AstToolError> {
    let preprocessor = Preprocessor::with_driver_options(
        source,
        path,
        crate::preprocess::files::FileServices {
            identity: super::file_identity::query,
            read: super::file_identity::read,
        },
        values,
        options,
    )
    .map_err(AstToolError::Initialization)?;
    emit_preprocessed(
        preprocessor,
        Destination::LowIr(output),
        1,
        budget,
        EmitMode::LowIr,
    )
}
fn emit_preprocessed(
    preprocessor: Preprocessor<'_>,
    destination: Destination<'_>,
    ordinal: usize,
    stack_budget: crate::support::stack::StackBudget,
    mode: EmitMode,
) -> Result<(), AstToolError> {
    let origin = if mode == EmitMode::LowIr {
        super::host_declarations::token_origin().map_err(AstToolError::Internal)?
    } else {
        0
    };
    let numbers = crate::lex::number::NumberOptions {
        dialect: crate::lex::number::NumberDialect::Gnu,
        binary_exponents: mode == EmitMode::LowIr,
    };
    let mut unit = SyntaxUnit::with_token_origin(preprocessor, origin, numbers);
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
        let analysis = crate::sema::analyze_unit(
            tree,
            cursor,
            declarations,
            stack_budget,
            matches!(mode, EmitMode::Semantics | EmitMode::LowIr),
            mode == EmitMode::LowIr,
        );
        Some(analysis.map_err(|error| match error {
            crate::sema::Error::Diagnostic { location, message }
            | crate::sema::Error::NonConstant { location, message }
            | crate::sema::Error::InstantiationDiagnostic { location, message } => {
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
                    | crate::sema::Error::NonConstant { location, message }
                    | crate::sema::Error::InstantiationDiagnostic { location, message },
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
                    crate::lowir::Error::Diagnostic(message) => {
                        AstToolError::Diagnostic(message.into_bytes())
                    }
                    crate::lowir::Error::ByteDiagnostic(bytes) => AstToolError::Diagnostic(bytes),
                    crate::lowir::Error::Construction(_) => {
                        AstToolError::Internal("typed LowIR construction failed")
                    }
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

fn lower_error(error: crate::lower::Error) -> AstToolError {
    match error {
        crate::lower::Error::Ir(error) => native_error(error),
        crate::lower::Error::Missing(group)
        | crate::lower::Error::Semantic(crate::sema::Error::Missing(group)) => {
            AstToolError::Missing(group)
        }
        _ => AstToolError::Internal("object startup finalization failed"),
    }
}
pub(crate) fn native_error(error: crate::lowir::Error) -> AstToolError {
    match error {
        crate::lowir::Error::NotImplemented(group) => AstToolError::Missing(group),
        crate::lowir::Error::Construction(_) => {
            AstToolError::Internal("native construction invariant")
        }
        crate::lowir::Error::Diagnostic(message) => AstToolError::Diagnostic(message.into_bytes()),
        crate::lowir::Error::ByteDiagnostic(bytes) => AstToolError::Diagnostic(bytes),
        crate::lowir::Error::Output(error) => AstToolError::Output(error),
        crate::lowir::Error::Limit => {
            AstToolError::Diagnostic(b"LowIR storage limit exceeded".to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::stack::StackBudget;

    #[test]
    fn host_incomplete_rtti_owns_descriptors_but_shares_name_objects() {
        use crate::lowir::{
            literals::Constant,
            model::{DataItem, GlobalInit},
            vocabulary::{Binding, Type},
        };
        let source = SourceBuffer::new(b"namespace std{class type_info;}struct A;const std::type_info*f(){return &typeid(A**);}".to_vec()).unwrap();
        let mut output = LowIrOutput::for_object_model(crate::native::ObjectModel::Host).unwrap();
        emit_compilation_unit(
            &source,
            b"incomplete-rtti",
            &PredefinedValues::default(),
            &crate::preprocess::options::DriverOptions::default(),
            &mut output,
            StackBudget::default(),
        )
        .unwrap();
        let program = &output.output.program;
        for (name, binding) in [
            ("_ZTI1A", Binding::Internal),
            ("_ZTIP1A", Binding::Internal),
            ("_ZTIPP1A", Binding::Internal),
            ("_ZTS1A", Binding::Weak),
            ("_ZTSP1A", Binding::Weak),
            ("_ZTSPP1A", Binding::Weak),
        ] {
            let global = program
                .globals()
                .iter()
                .find(|g| {
                    g.metadata
                        .object
                        .is_some_and(|object| program.names.get(object) == name)
                })
                .unwrap();
            assert!(global.metadata.binding == Some(binding), "{name}");
            if name.starts_with("_ZTIP") {
                let GlobalInit::Data(items) = &global.init else {
                    panic!("pointer descriptor must be defined")
                };
                assert!(matches!(
                    items[2],
                    DataItem::Constant(Type::I32, Constant::Integer(8))
                ));
            }
        }
        output.finish_object(b"incomplete-rtti".to_vec()).unwrap();
    }

    #[test]
    fn host_catch_copy_retires_throwing_default_argument_before_handler() {
        let source = SourceBuffer::new(b"struct T{T();~T()noexcept(false);};struct E{E(const E&,T=T());~E()noexcept(false);};void g();void f(){try{g();}catch(E e){g();}}".to_vec()).unwrap();
        let mut streamed = LowIrOutput::for_object_model(crate::native::ObjectModel::Host).unwrap();
        emit_compilation_unit(
            &source,
            b"host-catch-copy",
            &PredefinedValues::default(),
            &crate::preprocess::options::DriverOptions::default(),
            &mut streamed,
            StackBudget::default(),
        )
        .unwrap();
        streamed.finish_object(b"host-catch-copy".to_vec()).unwrap();
    }

    #[test]
    fn host_array_default_arguments_keep_one_region_state_on_every_edge() {
        for count in [3, 9] {
            let text = format!(
                "struct P{{P();~P();}};struct Q{{Q(P=P());~Q();}};struct L{{L();~L();}};void f(){{L before;Q a[{count}];}}"
            );
            let source = SourceBuffer::new(text.into_bytes()).unwrap();
            let mut output =
                LowIrOutput::for_object_model(crate::native::ObjectModel::Host).unwrap();
            emit_compilation_unit(
                &source,
                b"element-regions",
                &PredefinedValues::default(),
                &crate::preprocess::options::DriverOptions::default(),
                &mut output,
                StackBudget::default(),
            )
            .unwrap();
            assert!(
                output
                    .output
                    .program
                    .functions()
                    .iter()
                    .all(|f| f.body.is_none())
            );
            output.finish_object(b"element-regions".to_vec()).unwrap();
        }
    }

    #[test]
    fn host_declared_construction_joins_conditional_and_nested_allocation_regions() {
        let source = SourceBuffer::new(b"struct L{L(int);~L();};struct U{U();~U();};struct T{T(bool);};int g();int use(U*);void f(int c,int n){L before(0);L x(c?({L p(1);g();}):g());delete new T(use(new U[n])!=0);}".to_vec()).unwrap();
        let mut output = LowIrOutput::for_object_model(crate::native::ObjectModel::Host).unwrap();
        emit_compilation_unit(
            &source,
            b"construction-joins",
            &PredefinedValues::default(),
            &crate::preprocess::options::DriverOptions::default(),
            &mut output,
            StackBudget::default(),
        )
        .unwrap();
        output
            .finish_object(b"construction-joins".to_vec())
            .unwrap();
    }

    #[test]
    fn object_stream_releases_bodies_and_retains_forward_call_boundaries() {
        let source =
            SourceBuffer::new(b"int later(int); int main(){return later(7);}".to_vec()).unwrap();
        let mut output = LowIrOutput::for_object().unwrap();
        emit_compilation_unit(
            &source,
            b"object-stream",
            &PredefinedValues::default(),
            &crate::preprocess::options::DriverOptions::default(),
            &mut output,
            StackBudget::default(),
        )
        .unwrap();
        let program = &output.output.program;
        assert!(program.functions().iter().all(|f| f.body.is_none()));
        let callee = program
            .functions()
            .iter()
            .find(|f| program.names.get(f.name) == "later")
            .unwrap();
        let symbol = program.find_symbol(callee.name).unwrap();
        assert_eq!(program.forward_boundary(symbol), Some(callee.signature));
        assert!(program.has_definition_sink());
        assert!(output.output.definitions.is_empty());
        output.finish_object(b"object-stream".to_vec()).unwrap();
    }

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
    fn abort_role_coalesces_across_units_and_releases_forwarding_bodies() {
        use crate::lowir::{
            model::{Returns, Unwind},
            vocabulary::{Binding, Role},
        };
        let plain =
            "void plain(){__builtin_abort();}inline void unused(){__builtin_memmove(0,0,0);}";
        let guarded = "void may_throw();void guarded()noexcept{may_throw();}";
        for sources in [vec![plain], vec![plain, guarded], vec![guarded, plain]] {
            let forwarded = sources.len() == 2;
            let mut output = LowIrOutput::default();
            for (index, text) in sources.into_iter().enumerate() {
                let source = SourceBuffer::new(text.as_bytes().to_vec()).unwrap();
                emit_unit(
                    &source,
                    b"abort-boundaries",
                    &PredefinedValues::default(),
                    Destination::LowIr(&mut output),
                    index + 1,
                    StackBudget::default(),
                    EmitMode::LowIr,
                )
                .unwrap();
            }
            let program = &output.output.program;
            let abort = program
                .functions_in_symbol_order()
                .find(|function| program.names.get(function.name) == "__builtin_abort")
                .unwrap();
            let signature = program.signature(abort.signature);
            assert!(matches!(signature.boundary.unwind, Unwind::No));
            assert!(matches!(signature.boundary.returns, Returns::NoReturn));
            assert!(signature.parameters.is_empty());
            assert_eq!(
                abort.metadata.binding,
                Some(if forwarded {
                    Binding::Internal
                } else {
                    Binding::Strong
                })
            );
            assert_eq!(abort.metadata.role, (!forwarded).then_some(Role::Terminate));
            assert_eq!(
                abort.metadata.object.map(|name| program.names.get(name)),
                (!forwarded).then_some("abort")
            );
            assert_eq!(
                program
                    .functions_in_symbol_order()
                    .filter(|function| function.metadata.role == Some(Role::Terminate))
                    .count(),
                1
            );
            assert!(
                program
                    .functions()
                    .iter()
                    .all(|function| function.body.is_none())
            );
            assert!(
                !program
                    .functions()
                    .iter()
                    .any(|function| program.names.get(function.name).contains("memmove"))
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
    #[test]
    fn extended_cleanups_keep_distinct_declaring_owners_and_original_syntax() {
        for (source, per_owner) in [
            (b"int n;struct A{A(int){}~A(){++n;}};struct R{const A&r;};void f(){R a={A(1)};R b={A(2)};}".as_slice(), 1),
            (b"int n;struct A{A(int){}~A(){++n;}};struct R{A a;const A&r;};void f(){R a={A(1),A(1)};R b={A(2),A(2)};}".as_slice(), 2),
            (b"int n;struct A{int v;A(int i):v(i){}~A(){++n;}};int g(A a){return a.v;}void f(){const A&a=A(g(A(1)));const A&b=A(g(A(2)));}".as_slice(), 1),
            (b"int n;struct A{int v;A(int i):v(i){}~A(){++n;}};int g(const A&a){return a.v;}struct R{const A&r;int v;};void f(){R a={A(1),g(A(3))};R b={A(2),g(A(4))};}".as_slice(), 1),
        ] {
            let source = SourceBuffer::new(source.to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(
            crate::preprocess::unit::Preprocessor::with_name(&source, b"reference-ownership")
                .unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer =
            crate::sema::Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        crate::lower::assert_extended_ownership(&graph, tree, cursor, declarations, per_owner);
        assert_eq!(count, tree.expressions.mark());
        }
    }
    #[test]
    fn operand_queries_keep_constructor_parameters_without_a_result_temporary() {
        let source = crate::support::source::SourceBuffer::new(
            b"struct A{A(const A&)noexcept;~A()noexcept;};\
              struct E{E(A,A)noexcept;};\
              void f(A&a,A&b){E(a,b);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            crate::preprocess::unit::Preprocessor::with_name(&source, b"temporary-facts").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer =
            crate::sema::Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        crate::lower::assert_constructor_parameter_temporaries(&graph, tree, cursor, declarations);
        assert_eq!(tree.expressions.mark(), count);
    }
    fn anchor_classes(
        graph: &crate::sema::Graph,
        tree: &crate::syntax::ast::Tree,
        last: crate::syntax::ast::DeclId,
    ) -> Vec<crate::sema::lowering::EntityId> {
        use crate::{sema::lowering::TypeView, syntax::ast};
        let ast::DeclKind::Function { declarator, .. } = tree.declarations.get(last).kind else {
            panic!("anchor function");
        };
        let ty = graph
            .lower_entity_type(graph.lower_declared(declarator).unwrap())
            .unwrap();
        let TypeView::Function { parameters, .. } = graph.lower_type(ty).unwrap() else {
            panic!("anchor signature");
        };
        parameters
            .iter()
            .map(|&ty| {
                let TypeView::Pointer(ty) = graph.lower_type(ty).unwrap() else {
                    panic!("pointer");
                };
                let TypeView::Class(class) = graph.lower_type(ty).unwrap() else {
                    panic!("class");
                };
                class
            })
            .collect()
    }
    #[test]
    fn construction_contexts_keep_complete_offsets_without_emission_demand() {
        use crate::{
            preprocess::unit::Preprocessor, sema::Analyzer, support::source::SourceBuffer,
        };
        {
            let source = "struct V{virtual int f(){return 1;}int n;};struct P{virtual int p(){return 2;}int n;};struct B:virtual V{int n;};struct D:P,B{long long tail[3];};template<class T>struct U{virtual int f(){return T::missing();}};void use(){D x;}void anchor(D*,B*,V*,U<int>*){}".to_owned();
            let source = SourceBuffer::new(source.into_bytes()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"covariant-demand").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let syntax = (tree.expressions.mark(), tree.declarations.mark());
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let graph = analyzer.analyze(declarations).unwrap();
            let classes = anchor_classes(
                &graph,
                tree,
                *tree.decl_lists.get(declarations).last().unwrap(),
            );
            crate::lower::assert_construction_context(&graph, tree, cursor, &classes);
            assert_eq!(syntax, (tree.expressions.mark(), tree.declarations.mark()));
        }
    }
    #[test]
    fn covariant_routes_reuse_facts_without_publishing_tables_or_dormant_bodies() {
        use crate::{
            preprocess::unit::Preprocessor, sema::Analyzer, support::source::SourceBuffer,
        };
        for reference in [false, true] {
            let marker = if reference { "&" } else { "*" };
            let returned = if reference { "*value" } else { "value" };
            let source = format!(
                "struct P{{virtual int p();int n;}};struct R{{int r;}};\
                 struct H:P,R{{}};struct S:virtual H{{}};\
                 template<class T>struct Unused{{virtual int f(){{return T::missing();}}}};\
                 extern S*value;struct B{{virtual R{marker}get();}};struct D:B{{S{marker}get(){{return {returned};}}}};\
                 S{marker}use(D&p){{D local;return p.get();}}void anchor(D*,Unused<int>*){{}}"
            );
            let source = SourceBuffer::new(source.into_bytes()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"covariant-demand").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let syntax = (tree.expressions.mark(), tree.declarations.mark());
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let graph = analyzer.analyze(declarations).unwrap();
            let classes = anchor_classes(
                &graph,
                tree,
                *tree.decl_lists.get(declarations).last().unwrap(),
            );
            crate::lower::assert_covariant_routes(&graph, tree, cursor, &classes, reference);
            assert_eq!(syntax, (tree.expressions.mark(), tree.declarations.mark()));
        }
    }
    #[test]
    fn table_demand_reuses_identity_and_leaves_unrelated_classes_untouched() {
        use crate::{
            preprocess::unit::Preprocessor,
            sema::{Analyzer, lowering::TypeView},
            support::source::SourceBuffer,
            syntax::ast,
        };
        let source = SourceBuffer::new(b"struct A{virtual int z(){return 1;}virtual int f(int)=0;};struct B:A{int f(int){return 2;}};struct Unused{virtual int absent();};int main(){B b;return b.f(0);}void anchor(A*,B*,Unused*){}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"table-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax = (tree.expressions.mark(), tree.declarations.mark());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let ast::DeclKind::Function { declarator, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("anchor function");
        };
        let anchor = graph
            .lower_function(graph.lower_declared(declarator).unwrap())
            .unwrap();
        let TypeView::Function { parameters, .. } = graph.lower_type(anchor.ty).unwrap() else {
            panic!("anchor signature");
        };
        let classes: Vec<_> = parameters
            .iter()
            .map(|&ty| {
                let TypeView::Pointer(ty) = graph.lower_type(ty).unwrap() else {
                    panic!("anchor pointer");
                };
                let TypeView::Class(class) = graph.lower_type(ty).unwrap() else {
                    panic!("anchor class");
                };
                class
            })
            .collect();
        crate::lower::assert_table_demand(&graph, tree, cursor, &classes);
        assert_eq!(syntax, (tree.expressions.mark(), tree.declarations.mark()));
    }
    #[test]
    fn hierarchy_flags_distinguish_shared_and_repeated_occurrences_without_additional_demand() {
        use crate::{
            preprocess::unit::Preprocessor,
            sema::{Analyzer, lowering::TypeView},
            support::source::SourceBuffer,
            syntax::ast,
        };
        let source = SourceBuffer::new(
            br"
            struct Root{virtual void f();};
            struct Left:Root{};struct Right:Root{};
            struct Repeat:Left,Right{};
            struct VLeft:virtual Left{};struct VRight:virtual Left{};
            struct Diamond:VLeft,VRight{};
            struct Mixed:Diamond,Left{};
            struct Branch:virtual Right{};
            struct Both:Diamond,Branch{};
            struct Unrelated{virtual void unused();};
            void anchor(Root*,Repeat*,Diamond*,Mixed*,Both*,Unrelated*);
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"rtti-hierarchy-flags").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax = (tree.expressions.mark(), tree.declarations.mark());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let ast::DeclKind::Simple { declarators, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("anchor declaration");
        };
        let declarator = tree.init_declarators.get(declarators)[0].declarator;
        let function = graph.lower_declared(declarator).unwrap();
        let ty = graph.lower_function(function).unwrap().ty;
        let TypeView::Function { parameters, .. } = graph.lower_type(ty).unwrap() else {
            panic!("anchor signature");
        };
        crate::lower::assert_hierarchy_flags(&graph, tree, cursor, parameters);
        assert_eq!(syntax, (tree.expressions.mark(), tree.declarations.mark()));
    }
    #[test]
    fn route_and_row_queries_reuse_facts_without_demanding_tables_or_bodies() {
        use crate::{
            sema::{Analyzer, lowering::TypeView},
            syntax::ast,
        };
        let source = SourceBuffer::new(
            br"
            struct Inside { int n; };
            struct V : Inside { virtual int f(); virtual int g() const; virtual ~V(); };
            struct B : virtual V { int b; };
            struct W { int w; };
            struct D : B, virtual W { int d; };
            template<class T> struct Dormant : virtual V { void bad(){T::missing();} };
            struct P { virtual void f(); virtual void f(int); virtual ~P(); };
            struct Q : virtual P {};
            struct R : Q, virtual W {};
            int sizes[sizeof(D)+sizeof(R)];
            void anchor(D*, B*, V*, Inside*, W*, Dormant<int>*, R*, Q*, P*) {}
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"projection-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax_count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let ast::DeclKind::Function { declarator, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("anchor");
        };
        let entity = graph.lower_declared(declarator).unwrap();
        let TypeView::Function { parameters, .. } = graph
            .lower_type(graph.lower_function(entity).unwrap().ty)
            .unwrap()
        else {
            panic!("anchor signature");
        };
        let classes: Vec<_> = parameters
            .iter()
            .map(|&ty| {
                let TypeView::Pointer(ty) = graph.lower_type(ty).unwrap() else {
                    panic!("pointer");
                };
                let TypeView::Class(class) = graph.lower_type(ty).unwrap() else {
                    panic!("class");
                };
                class
            })
            .collect();
        crate::lower::assert_base_routes(&graph, tree, cursor, &classes);
        assert_eq!(syntax_count, tree.expressions.mark());
    }
}
