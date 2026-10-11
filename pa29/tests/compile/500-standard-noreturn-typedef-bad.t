// AUDIT-ID: ATTR-NORETURN
// AUDIT-EXPECT: reject
// N3485 7.6.3p1: noreturn applies only to the declarator-id of a function
// declaration; a typedef declares no function.
[[noreturn]] typedef void handler();
