enum class A : unsigned char {
  x = 2,
};

enum class B : unsigned char {
  y = static_cast<unsigned char>(A::x),
};

static_assert(A::x == A::x, "ok");
