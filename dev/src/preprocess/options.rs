// Translated from preprocess/preprocessor.cpp::PredefinedText. See NOTICE.
//! Invocation inputs are captured by the tool and shared by its primary sources.

#[derive(Default)]
pub struct PredefinedValues {
    pub author: Vec<u8>,
    pub date: Vec<u8>,
    pub time: Vec<u8>,
}
impl PredefinedValues {
    pub(crate) fn text(&self) -> Vec<u8> {
        let mut text = b"#define __CPPGM__ 201303L\n#define __cplusplus 201103L\n#define __STDC_HOSTED__ 1\n#define __x86_64__ 1\n#define __linux__ 1\n".to_vec();
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
    pub host_predefines: Vec<u8>,
    pub command_line: Vec<u8>,
    pub include_paths: Vec<Vec<u8>>,
    pub system_include_paths: Vec<Vec<u8>>,
}
