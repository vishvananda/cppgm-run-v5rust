// HHC-177
typedef unsigned long size_t;

int f(int x) {
  union {
    int t;
    size_t a;
  } u;
  u.t = x;
  return (int)u.a;
}
