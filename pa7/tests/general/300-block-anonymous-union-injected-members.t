int f(int x) {
  union {
    int t;
    long a;
  };
  t = x;
  return (int)a;
}
