struct S {
  int x;
  union {
    int t;
    long a;
  };
  int y;
};

int f(S* s, int v) {
  s->t = v;
  return (int)s->a;
}
