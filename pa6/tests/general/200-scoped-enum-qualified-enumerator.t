enum class A : unsigned char {
  X = 1,
  Y = 2
};

enum class B : unsigned char {
  X = A::X,
  Y = A::Y
};
