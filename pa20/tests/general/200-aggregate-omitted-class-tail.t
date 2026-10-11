struct Inner {
  int value;
  Inner() : value(7) {}
};

struct Outer {
  int first;
  Inner second;
};

Outer make_partial(int x) {
  return {x};
}

Outer make_empty() {
  return {};
}

int main() {
  Outer a = make_partial(5);
  Outer b = make_empty();
  return (a.first == 5 &&
          a.second.value == 7 &&
          b.first == 0 &&
          b.second.value == 7) ? 0 : 1;
}
