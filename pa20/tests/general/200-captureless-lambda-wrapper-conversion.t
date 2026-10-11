struct Wrapper {
  int (*fp)(int);

  Wrapper(int (*p)(int)) : fp(p) {}

  int operator()(int x) const {
    return fp(x);
  }
};

int use(const Wrapper & w) {
  return w(4);
}

int main() {
  return use([](int x) { return x + 1; });
}
