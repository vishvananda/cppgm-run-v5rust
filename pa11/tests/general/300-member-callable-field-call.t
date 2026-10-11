struct Fn {
  int operator()(int x) const {
    return x + 1;
  }
};

struct Hooks {
  Fn f;
};

int g(int x) {
  Hooks h;
  return h.f(x);
}

int main() {
  return 0;
}
