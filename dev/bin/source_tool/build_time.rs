use cppgm_rust::preprocess::options::PredefinedValues;

// Host clock input, confined to the tool boundary. Python's standard time
// module uses the host's localtime, including libc's POSIX TZ rules. This
// avoids authored FFI and the differing timezone policies of date utilities.
// One query captures both fields for every primary source in this invocation.
pub(super) fn query() -> Result<PredefinedValues, String> {
    let stamp = std::process::Command::new("/usr/bin/python3")
        .args([
            "-I", "-S", "-c",
            "import time; stamp=time.asctime(time.localtime()); print(stamp[4:11]+stamp[20:24]); print(stamp[11:19])",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if !stamp.status.success() {
        return Err("cannot query the local build time".to_owned());
    }
    let Some(split) = stamp.stdout.iter().position(|&byte| byte == b'\n') else {
        return Err("invalid local build time".to_owned());
    };
    let date = &stamp.stdout[..split];
    let time = &stamp.stdout[split + 1..];
    Ok(PredefinedValues {
        author: b"Course Solution".to_vec(),
        date: date.to_vec(),
        time: time.strip_suffix(b"\n").unwrap_or(time).to_vec(),
    })
}
