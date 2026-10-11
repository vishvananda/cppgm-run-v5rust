// HHC-178
typedef unsigned long size_t;

size_t f(int x) {
  union {
    int t;
    size_t a;
  } u;
  u.t = x;
  return sizeof(u);
}

int main() { return (int)(f(3) - sizeof(int)); }
