int f(int x) {
  return __builtin_constant_p(x);
}

int g() {
  return __builtin_constant_p(3) + __builtin_constant_p(1 + 2);
}

enum E {
  A = 2,
  B = A + 3
};

int h() {
  int values[B];
  return __builtin_constant_p(B) + sizeof(values);
}
