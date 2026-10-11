struct S {
  float x;
  float y;
};

S g(float x);

void f(float x) {
  const struct S s = g(x);
  (void)s.x;
}
