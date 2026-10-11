struct B { virtual void f() = 0; };
struct D : virtual B {};
static_assert(__is_abstract(D), "");
static_assert(!__is_constructible(D), "");
