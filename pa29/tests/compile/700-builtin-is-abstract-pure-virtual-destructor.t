// VALIDATION: compile-pass
struct A { virtual ~A() = 0; };
static_assert(__is_abstract(A), "");
