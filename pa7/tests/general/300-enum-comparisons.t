enum E {
  A,
  B
};

bool f(E x) {
  return x == A || x != B || x < B;
}
