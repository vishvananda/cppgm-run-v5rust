// VALIDATION: compile-pass
// A nonviable converting constructor does not hide an exact conversion
// function to the destination class.
struct M {};
struct C { C(const M&); };
struct X { operator M() const; operator C() const; };
static_assert(__is_convertible(const X&, C), "");
