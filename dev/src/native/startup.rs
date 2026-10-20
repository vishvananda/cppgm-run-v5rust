//! Startup roles and external atexit discovery in symbol order. See NOTICE.
use crate::lowir::{
    Error,
    model::{Arity, Program, SymbolId},
    vocabulary::{Role, Type},
};
use std::io::Write;

#[derive(Default)]
pub(super) struct Startup {
    pub(super) entry: Option<SymbolId>,
    pub(super) init: Option<SymbolId>,
    pub(super) fini: Option<SymbolId>,
    pub(super) atexit: Option<SymbolId>,
    pub(super) entry_parameters: usize,
}

impl Startup {
    pub(super) fn find(program: &Program) -> Result<Self, Error> {
        let mut startup = Self::default();
        let mut defines_atexit = false;
        for symbol in program.symbols() {
            let Some(function) = program.function(symbol) else {
                continue;
            };
            let id = program
                .find_symbol(symbol.name)
                .ok_or_else(|| Error::Construction("function without a symbol identity".into()))?;
            let metadata = &function.metadata;
            let object = program.names.bytes(metadata.object.unwrap_or(symbol.name));
            let signature = program.signature(function.signature);
            if function.body.is_none() {
                if metadata.tls_for.is_none()
                    && object == b"atexit"
                    && signature.boundary.arity == Arity::Fixed
                    && signature.result == Type::I32
                    && matches!(signature.parameters.as_slice(), [param] if param.ty == Type::Ptr)
                {
                    startup.atexit = Some(id);
                }
                continue;
            }
            defines_atexit |= object == b"atexit";
            match metadata.role {
                Some(Role::Entry) => {
                    startup.entry = Some(id);
                    startup.entry_parameters = signature.parameters.len();
                }
                Some(Role::Init) => startup.init = Some(id),
                Some(Role::Fini) => startup.fini = Some(id),
                _ => {}
            }
        }
        if defines_atexit {
            startup.atexit = None;
        }
        Ok(startup)
    }

    pub(super) fn write(&self, program: &Program, out: &mut impl Write) -> Result<(), Error> {
        let write = |out: &mut dyn Write| -> std::io::Result<()> {
            out.write_all(b"machine_ir x86_64 linux\n")?;
            let Some(entry) = self.entry else {
                return Ok(());
            };
            out.write_all(b"\nstartup\n")?;
            if let Some(init) = self.init {
                out.write_all(b"    call @")?;
                out.write_all(program.names.bytes(program.symbol(init).name))?;
                out.write_all(b" [args=()]\n")?;
            }
            out.write_all(b"    call @")?;
            out.write_all(program.names.bytes(program.symbol(entry).name))?;
            out.write_all(match self.entry_parameters {
                0 => b" [args=()]\n",
                1 => b" [args=(rdi)]\n",
                _ => b" [args=(rsi,rdi)]\n",
            })?;
            if self.fini.is_some() || self.atexit.is_some() {
                out.write_all(b"    mov r12, rax\n")?;
                if self.atexit.is_some() {
                    out.write_all(b"    atexit_drain\n")?;
                }
                if let Some(fini) = self.fini {
                    out.write_all(b"    call @")?;
                    out.write_all(program.names.bytes(program.symbol(fini).name))?;
                    out.write_all(b" [args=()]\n")?;
                }
                out.write_all(b"    mov rdi, r12\n    exit\n")
            } else {
                out.write_all(b"    mov rdi, rax\n    exit\n")
            }
        };
        write(out).map_err(Error::Output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Body, Boundary, Function, Metadata, Parameter, Signature},
    };

    fn add_function(
        program: &mut Program,
        name: &str,
        object: Option<&str>,
        parameters: Vec<Parameter>,
        result: Type,
        body: bool,
    ) -> Result<(), Error> {
        let name = program.names.intern(name)?;
        let object = object.map(|name| program.names.intern(name)).transpose()?;
        let source = program.add_source(std::path::Path::new(""))?;
        let symbol = program.reference(name, source, 0)?;
        let signature = program.add_signature(Signature {
            parameters,
            result,
            boundary: Boundary::default(),
        })?;
        program.add(
            symbol,
            Function {
                name,
                signature,
                metadata: Metadata {
                    object,
                    ..Metadata::default()
                },
                body: body.then(|| Body::new(None, 0)),
                location: None,
            },
        )
    }

    #[test]
    fn object_named_atexit_definition_suppresses_runtime_declaration() -> Result<(), Error> {
        let mut program = Program::default();
        let parameter = Parameter {
            name: program.names.intern("callback")?,
            ty: Type::Ptr,
            pass: None,
            no_alias: false,
            object_bytes: 0,
            classes: None,
        };
        add_function(
            &mut program,
            "register",
            Some("atexit"),
            vec![parameter],
            Type::I32,
            false,
        )?;
        assert!(Startup::find(&program)?.atexit.is_some());
        // Suppression uses the definition's object name regardless of its type.
        add_function(
            &mut program,
            "replacement",
            Some("atexit"),
            vec![],
            Type::Void,
            true,
        )?;
        assert!(Startup::find(&program)?.atexit.is_none());
        Ok(())
    }

    #[test]
    fn explicit_startup_roles_use_their_typed_parameter_counts() -> Result<(), Error> {
        let mut program = Program::default();
        for (name, role, params) in [
            ("begin", Role::Init, &[][..]),
            (
                "entry",
                Role::Entry,
                &[("argc", Type::I32), ("argv", Type::Ptr)][..],
            ),
            ("end", Role::Fini, &[][..]),
        ] {
            let mut builder = FunctionBuilder::new(&mut program, name, params, Type::Void)?;
            builder.metadata(Metadata {
                role: Some(role),
                ..Metadata::default()
            });
            builder.finish()?;
        }
        let mut text = Vec::new();
        Startup::find(&program)?.write(&program, &mut text)?;
        assert_eq!(text, b"machine_ir x86_64 linux\n\nstartup\n    call @begin [args=()]\n    call @entry [args=(rsi,rdi)]\n    mov r12, rax\n    call @end [args=()]\n    mov rdi, r12\n    exit\n");
        Ok(())
    }
}
