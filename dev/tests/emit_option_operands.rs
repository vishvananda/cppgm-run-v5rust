//! Emit options retain operand ownership through preprocessing and lowering.
use std::process::Command;

#[test]
fn include_and_output_operands_cannot_select_optimization_or_object_dialect()
-> Result<(), Box<dyn std::error::Error>> {
    let directory =
        std::env::temp_dir().join(format!("cppgm-emit-operands-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let source = directory.join("unit.cpp");
    std::fs::write(
        &source,
        b"
#ifdef __OPTIMIZE__
int selected(int value) { return value + 1; }
#else
int selected(int value) { return value + 2; }
#endif
",
    )?;
    let mut baseline = None;
    for (options, output) in [
        (vec!["-O0"], "output.lowir"),
        (vec!["-O0", "-I", "-O3"], "output.lowir"),
        (vec!["-I", "-g0"], "output.lowir"),
        (vec!["-O0"], "-O3"),
        (vec!["-O0", "-I", "--emit-lowir", "-O3"], "output.lowir"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_cppgm"))
            .current_dir(&directory)
            .arg("--emit-lowir")
            .args(&options)
            .args(["-o", output])
            .arg(&source)
            .output()?;
        assert!(result.status.success(), "{options:?}: {:?}", result.stderr);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty(), "{options:?}: {:?}", result.stderr);
        let bytes = std::fs::read(directory.join(output))?;
        if let Some(expected) = &baseline {
            assert_eq!(&bytes, expected, "{options:?} -o {output}");
        } else {
            baseline = Some(bytes);
        }
    }
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
