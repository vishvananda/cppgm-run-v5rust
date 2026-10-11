// HHC-171
template<class T>
struct pair {
  pair(pair const&) = default;
  pair(pair&&) = default;

  template<class U1, class U2, int = 0>
  pair(U1&&, U2&&) {}
};

pair<int> make_pair_int() {
  return pair<int>(1, 2);
}
