//! Host `LowIR`'s raw SSEUP class must preserve both halves of a variadic value.
use std::{path::PathBuf, process::Command};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn checked(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let output = command.output()?;
    assert!(output.status.success(), "{command:?}: {output:?}");
    assert!(output.stdout.is_empty(), "{command:?}: {output:?}");
    Ok(())
}
#[test]
fn raw_and_stated_sseup_arguments_preserve_the_complete_host_value()
-> Result<(), Box<dyn std::error::Error>> {
    let directory =
        Scratch(std::env::temp_dir().join(format!("cppgm-variadic-sseup-{}", std::process::id())));
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.lowir");
    let object = directory.0.join("input.o");
    let host = directory.0.join("host.cpp");
    let program = directory.0.join("program");
    std::fs::write(
        &host,
        b"struct Pair { unsigned long low, high; };\n\
        extern \"C\" Pair read_value(long, ...);\n\
        int main() {\n\
          union { __float128 value; Pair bits; } input;\n\
          input.value = (__float128)7.25;\n\
          Pair result = read_value(0L, input.value);\n\
          return result.low != input.bits.low || result.high != input.bits.high;\n\
        }\n",
    )?;
    for classes in [14, 142] {
        std::fs::write(
            &source,
            format!(
                "function @read_value(%tag:i64)->obj<16x8> [arity=variadic] {{
                 slot $ap:obj<24x8>
                 block ^entry:
                 %ap=addr $ap
                 va_start %ap
                 %value=va_arg obj<16x8> %ap [classes={classes}]
                 return obj<16x8> %value
                 }}\n"
            ),
        )?;
        for level in ["-O0", "-O2"] {
            checked(
                Command::new(env!("CARGO_BIN_EXE_cppgm"))
                    .args([level, "-c", "-o"])
                    .arg(&object)
                    .arg(&source),
            )?;
            checked(
                Command::new("g++")
                    .arg(&host)
                    .arg(&object)
                    .arg("-o")
                    .arg(&program),
            )?;
            checked(&mut Command::new(&program))?;
        }
    }
    Ok(())
}
