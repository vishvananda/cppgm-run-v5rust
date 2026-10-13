fn main() -> std::process::ExitCode {
    cppgm_rust::driver::lowir::run(&std::env::args_os().skip(1).collect::<Vec<_>>(), peak_rss)
}
fn peak_rss() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmHWM:")?
                    .split_whitespace()
                    .next()?
                    .parse()
                    .ok()
            })
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use cppgm_rust::lowir::{self, Error};
    use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn byte_paths_survive_parser_and_deferred_diagnostics() -> std::io::Result<()> {
        let directory =
            Scratch(std::env::temp_dir().join(format!("v5rust-pa8-paths-{}", std::process::id())));
        std::fs::create_dir(&directory.0)?;
        let path = directory
            .0
            .join(OsString::from_vec(b"source-\xff.lowir".to_vec()));
        let output = directory
            .0
            .join(OsString::from_vec(b"output-\xff.lowir".to_vec()));
        for (source, expected) in [
            ("?", "unexpected character"),
            (
                "function @f() -> void { block ^e: call void @missing() return void }",
                "undefined top-level symbol: missing",
            ),
            (
                "function @f() -> i64 { block ^e: return void }",
                "return type does not match function in function @f",
            ),
        ] {
            std::fs::write(&path, source)?;
            let result = lowir::roundtrip(std::slice::from_ref(&path), &output, false);
            let mut expected = expected.as_bytes().to_vec();
            expected.extend_from_slice(b" at ");
            expected.extend_from_slice(path.as_os_str().as_encoded_bytes());
            expected.extend_from_slice(b":1");
            assert!(matches!(result, Err(Error::ByteDiagnostic(message)) if message == expected));
        }
        std::fs::write(&path, "function @f() -> void { block ^e: return void }\n")?;
        let result = lowir::roundtrip(std::slice::from_ref(&path), &output, true);
        assert!(result.is_ok());
        assert_eq!(
            std::fs::read(&output)?,
            b"function @f() -> void {\n  block ^e:\n    return void\n}\n"
        );
        Ok(())
    }
}
