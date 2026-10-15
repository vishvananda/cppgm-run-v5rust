// expect reject
// The contextual conversion is valid, but its non-constexpr call is not a
// constant exception specification. This is an ordinary rejection, not a stub.
struct S { operator bool() const; };
void f() noexcept(!S());
