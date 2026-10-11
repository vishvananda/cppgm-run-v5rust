// VALIDATION: compile-pass
struct __attribute__((__aligned__(8))) A {};
static_assert(__alignof__(A) == 8, "");
