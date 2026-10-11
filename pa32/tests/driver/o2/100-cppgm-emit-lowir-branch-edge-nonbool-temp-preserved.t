#include <cstdio>

struct D {
  void operator()(int *p) const { std::printf("%p\n", (void *)p); }
};

struct P {
  int *p;
  D d;

  P(int *q) : p(q) {}

  void reset(int *q) {
    int *old = p;
    p = q;
    if(old) d(old);
  }

  ~P() { reset(0); }
};

int main() { P p((int *)0x1234); }
