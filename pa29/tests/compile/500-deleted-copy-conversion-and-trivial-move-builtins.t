struct X { X(const X&) = delete; X(X&&) = default; };
struct Y { operator X&(); };
static_assert(!__is_convertible(Y, X), "");
static_assert(__is_trivially_constructible(X, X&&), "");
