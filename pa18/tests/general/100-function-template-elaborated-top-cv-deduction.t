struct Foo {
  int value;
};

template<class T>
const T& pick_first(const T& lhs, const T& rhs) {
  return lhs.value ? lhs : rhs;
}

int main() {
  const struct Foo lhs = {7};
  const Foo rhs = {3};
  return pick_first(lhs, rhs).value == 7 ? 0 : 1;
}
