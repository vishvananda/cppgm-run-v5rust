enum class A {
  invalid,
  other
};

enum class B {
  invalid,
  other
};

using C = A;

static_assert(A::invalid == A::invalid, "ok");
static_assert(A::other == A::other, "ok");
static_assert(B::invalid == B::invalid, "ok");
static_assert(C::invalid == A::invalid, "ok");
