// AUDIT: a member initializer may instantiate class templates while the
// enclosing constexpr constructor is being evaluated, so the constructor's own
// member and base list must survive that instantiation.
template <int N> struct chain {
  chain<N - 1> next;
  int depth;
  constexpr chain() : depth(N) {}
};

template <> struct chain<0> {
  int depth;
  constexpr chain() : depth(0) {}
};

template <int N> struct measure {
  static constexpr int of() { return chain<N>().depth; }
};

struct marker {
  int flag;
  constexpr marker() : flag(1) {}
};

struct item : marker {
  int first;
  int second;
  constexpr item(int a) : marker(), first(a), second(measure<10>::of()) {}
};

constexpr item table[] = { item(1), item(2) };

int main() { return table[1].second - 10 + table[0].first - 1; }
