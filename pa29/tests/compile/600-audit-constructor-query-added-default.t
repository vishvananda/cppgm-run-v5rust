struct LaterDefault { LaterDefault(int) noexcept; };
static_assert(!__has_nothrow_constructor(LaterDefault), "no default yet");
static_assert(!__has_trivial_constructor(LaterDefault), "repeat absent query");
LaterDefault::LaterDefault(int = 0) noexcept {}
static_assert(__has_nothrow_constructor(LaterDefault), "later default is visible");
static_assert(!__has_trivial_constructor(LaterDefault), "user provided");
static_assert(__is_nothrow_constructible(LaterDefault), "ordinary selection agrees");

int throwing();
struct PartialDefaults { PartialDefaults(int, int) noexcept; };
static_assert(!__has_nothrow_constructor(PartialDefaults), "two arguments required");
PartialDefaults::PartialDefaults(int, int = 0) noexcept {}
static_assert(!__has_nothrow_constructor(PartialDefaults), "one argument still required");
PartialDefaults::PartialDefaults(int = throwing(), int) noexcept;
static_assert(__has_nothrow_constructor(PartialDefaults), "legacy query reads function exceptions");
static_assert(!__is_nothrow_constructible(PartialDefaults), "construction evaluates the default");

int main() { return 0; }
