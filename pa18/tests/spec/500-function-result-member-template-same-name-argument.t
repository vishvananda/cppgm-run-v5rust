// N3485 14.8.2 [temp.deduct]: substitute a function-template result once.
template<class> struct wrapper {};
struct E {};

template<class>
struct owner {
  template<class E>
  static E const& set();
};

int main() {
  wrapper<E> const& result =
      owner<int>::template set<wrapper<E> >();
  (void)result;
  return 0;
}
// VALIDATION: compile-pass
