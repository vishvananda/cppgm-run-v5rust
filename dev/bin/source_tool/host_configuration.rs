//! Capture host configuration from empty-input queries, as `gen_builtin_host_config.pl` does.
//! No user source is passed to the host compiler.
use std::{
    error::Error,
    process::{Command, Output, Stdio},
};

fn words(text: &str) -> Result<Vec<String>, &'static str> {
    let mut result = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut started = false;
    for ch in text.chars() {
        if escaped {
            word.push(ch);
            escaped = false;
        } else if ch == '\\' && quote != Some('\'') {
            escaped = true;
            started = true;
        } else if quote == Some(ch) {
            quote = None;
        } else if quote.is_none() && matches!(ch, '\'' | '"') {
            quote = Some(ch);
            started = true;
        } else if quote.is_none() && ch.is_whitespace() {
            if started {
                result.push(std::mem::take(&mut word));
                started = false;
            }
        } else {
            word.push(ch);
            started = true;
        }
    }
    if escaped || quote.is_some() {
        return Err("unterminated host configuration word");
    }
    if started {
        result.push(word);
    }
    Ok(result)
}

fn probe(base: &[String], args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let (program, flags) = base.split_first().ok_or("empty host compiler command")?;
    let output = Command::new(program)
        .args(flags)
        .args(args)
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "host configuration probe {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output)
}

pub(super) struct Configuration {
    pub macros: Vec<u8>,
    pub strict_macros: Vec<u8>,
    pub includes: Vec<u8>,
}

pub(super) fn query() -> Result<Configuration, Box<dyn Error>> {
    let mut base = words(option_env!("CPPGM_BUILD_HOST_CXX").unwrap_or("c++"))?;
    base.extend(words(option_env!("CPPGM_STDLIB_FLAGS").unwrap_or(""))?);
    let macros = probe(&base, &["-std=gnu++11", "-dM", "-E", "-x", "c++", "-"])?;
    let strict_macros = probe(&base, &["-std=c++11", "-dM", "-E", "-x", "c++", "-"])?;
    let paths = probe(&base, &["-E", "-x", "c++", "-", "-v"])?;
    let mut collecting = false;
    let mut includes = Vec::new();
    for line in String::from_utf8(paths.stderr)?.lines() {
        if line.contains("#include <...> search starts here:") {
            collecting = true;
            continue;
        }
        if !collecting {
            continue;
        }
        if line.contains("End of search list.") {
            break;
        }
        let path = line.trim();
        if !path.is_empty()
            && !path.contains("(framework directory)")
            && !includes.contains(&path.to_owned())
        {
            includes.push(path.to_owned());
        }
    }
    Ok(Configuration {
        macros: macros.stdout,
        strict_macros: strict_macros.stdout,
        includes: includes.join("\n").into_bytes(),
    })
}
