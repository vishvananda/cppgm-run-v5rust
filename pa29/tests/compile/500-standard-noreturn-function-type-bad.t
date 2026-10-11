// AUDIT-ID: ATTR-NORETURN
// AUDIT-EXPECT: reject
// N3485 7.6.3p1, 8.3.5p1: after the parameter list the attribute appertains
// to the function type, not to the declarator-id.
void stop() [[noreturn]];
