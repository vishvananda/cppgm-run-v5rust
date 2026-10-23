// Translated from preprocess/preprocessor.cpp::PredefinedText. See NOTICE.
//! Invocation inputs are captured by the tool and shared by its primary sources.

#[derive(Default)]
pub struct PredefinedValues {
    pub author: Vec<u8>,
    pub date: Vec<u8>,
    pub time: Vec<u8>,
}
impl PredefinedValues {
    pub(crate) fn text(&self, standard: LanguageStandard) -> Vec<u8> {
        let mut text = b"#define __CPPGM__ 201303L\n#define __cplusplus ".to_vec();
        text.extend_from_slice(standard.macro_value());
        text.extend_from_slice(
            b"\n#define __STDC_HOSTED__ 1\n#define __x86_64__ 1\n#define __linux__ 1\n",
        );
        for (name, value) in [
            (b"__CPPGM_AUTHOR__".as_slice(), &self.author),
            (b"__DATE__", &self.date),
            (b"__TIME__", &self.time),
        ] {
            text.extend_from_slice(b"#define ");
            text.extend_from_slice(name);
            text.extend_from_slice(b" \"");
            for &byte in value {
                if matches!(byte, b'"' | b'\\') {
                    text.push(b'\\');
                }
                text.push(byte);
            }
            text.extend_from_slice(b"\"\n");
        }
        text
    }
}

/// Driver preprocessing inputs captured once and borrowed by each translation unit.
#[derive(Default)]
pub struct DriverOptions {
    pub hosted: bool,
    pub standard: LanguageStandard,
    pub strict: bool,
    pub modes: FeatureModes,
    pub host_predefines: Vec<u8>,
    pub command_line: Vec<u8>,
    pub include_paths: Vec<Vec<u8>>,
    pub system_include_paths: Vec<Vec<u8>>,
}

/// Language guarantees used by hosted preprocessing probes.
#[derive(Clone, Copy)]
pub struct FeatureModes {
    pub exceptions: bool,
    pub rtti: bool,
}
impl Default for FeatureModes {
    fn default() -> Self {
        Self {
            exceptions: true,
            rtti: true,
        }
    }
}

/// Standard selection is a macro/declaration mode, separate from GNU literals.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum LanguageStandard {
    #[default]
    Cxx11,
    Cxx14,
    Cxx17,
}
impl LanguageStandard {
    pub(crate) fn macro_value(self) -> &'static [u8] {
        match self {
            Self::Cxx11 => b"201103L",
            Self::Cxx14 => b"201402L",
            Self::Cxx17 => b"201703L",
        }
    }
    pub(crate) fn parse(name: &[u8]) -> Option<(Self, bool)> {
        let (version, strict) = if let Some(version) = name.strip_prefix(b"gnu++") {
            (version, false)
        } else {
            (name.strip_prefix(b"c++")?, true)
        };
        Some((
            match version {
                b"11" | b"0x" => Self::Cxx11,
                b"14" | b"1y" => Self::Cxx14,
                b"17" | b"1z" => Self::Cxx17,
                _ => return None,
            },
            strict,
        ))
    }
}
