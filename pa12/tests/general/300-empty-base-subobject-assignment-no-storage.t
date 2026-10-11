struct EmptyBase {};

struct EmptyDerived : EmptyBase {
  EmptyDerived &operator=(const EmptyDerived &) = default;
};

struct Holder : EmptyDerived {
  int value;
};

int main() {
  Holder src;
  Holder dst;
  src.value = 0x12345678;
  dst.value = 0x7f7f7f7f;
  EmptyDerived &lhs = dst;
  const EmptyDerived &rhs = src;
  lhs = rhs;
  return dst.value == 0x7f7f7f7f ? 0 : 1;
}
