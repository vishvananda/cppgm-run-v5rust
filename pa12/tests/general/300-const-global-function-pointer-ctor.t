struct H {
  H(void (*p_in)()) : p(p_in) {}
  void (*p)() = 0;
};

void f() {}

const H h = {&f};

int main() {
  return h.p == &f ? 0 : 1;
}
