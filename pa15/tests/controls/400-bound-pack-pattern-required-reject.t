// sizeof... counts the whole pack; it cannot supply an expansion's elements. expect reject
int take(int n) { return n; }
template<int... N> struct Values {
  int run() { return take((sizeof...(N))...); }
};
int main() { Values<1, 2> value; return value.run(); }
