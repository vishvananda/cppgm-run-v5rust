// HHC-137
struct Tag {};
struct Value {};

template<class T>
struct Base {
  Base(Tag, T const&, Value) {}
};

template<class... Ts>
struct X : Base<int> {
  template<class A>
  X(A const& a) : Base<int>(Tag{}, a, Value{}) {}
};
