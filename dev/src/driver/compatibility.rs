//! Hosted driver flag policy from cppgm++.cpp and driver/toolchain.cpp. See NOTICE.
use super::{ast::AstToolError, host_environment::HostEnvironment};
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

pub(super) fn outer_flag(bytes: &[u8]) -> bool {
    bytes.starts_with(b"-W")
        || bytes.starts_with(b"-f")
        || bytes.starts_with(b"-m")
        || matches!(bytes, b"-pipe" | b"-w" | b"-pg" | b"-MD" | b"-MMD" | b"-MP")
}

fn refused(bytes: &[u8]) -> bool {
    matches!(
        bytes,
        b"-funsigned-char"
            | b"-fno-signed-char"
            | b"-fshort-wchar"
            | b"-fshort-enums"
            | b"-fpack-struct"
            | b"-funsigned-bitfields"
            | b"-fno-signed-bitfields"
            | b"-fno-elide-constructors"
            | b"-ffreestanding"
            | b"-fno-hosted"
            | b"-m32"
            | b"-mx32"
            | b"-m16"
            | b"-mlong-double-64"
            | b"-mlong-double-128"
            | b"-msoft-float"
            | b"-mno-sse"
            | b"-mno-sse2"
            | b"-mno-80387"
            | b"-mno-fp-ret-in-387"
            | b"-mfpmath=387"
    ) || bytes.starts_with(b"-fpack-struct=")
        || bytes.starts_with(b"-mabi=")
}

pub(super) fn validate(arg: &OsStr, host: &HostEnvironment<'_>) -> Result<(), AstToolError> {
    let bytes = arg.as_bytes();
    let prefix = if let Some(name) = bytes.strip_prefix(b"-stdlib=") {
        if host.stdlib_selected(name) {
            return Ok(());
        }
        return Err(AstToolError::Diagnostic(
            [
                b"this compiler was not built for ".as_slice(),
                bytes,
                b" (CPPGM_STDLIB_FLAGS)",
            ]
            .concat(),
        ));
    } else if refused(bytes) {
        b"unsupported option (it changes the program's meaning): ".as_slice()
    } else if matches!(bytes, b"-pg" | b"-std" | b"-stdlib") {
        b"unsupported option: "
    } else {
        return Ok(());
    };
    Err(AstToolError::Diagnostic([prefix, bytes].concat()))
}
