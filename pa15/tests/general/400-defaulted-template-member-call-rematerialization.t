struct Root {
  int operator()(int value) const { return value; }
};

template<class Prev, class T, class Storage = T const&>
struct Lazy {
  Prev const& prev;
  Storage value;

  Lazy(Prev const& p, T const& v) : prev(p), value(v) {}

  int operator()(int input) const {
    return prev(input) + value;
  }
};

template<class T>
Lazy<Root, T> append(Root const& prev, T const& value) {
  return Lazy<Root, T>(prev, value);
}

template<class PrevPrev, class TPrev, class T>
Lazy<Lazy<PrevPrev, TPrev>, T> append(Lazy<PrevPrev, TPrev> const& prev, T const& value) {
  typedef Lazy<PrevPrev, TPrev> Prev;
  return Lazy<Prev, T>(prev, value);
}

int use(int input) {
  Root root;
  return append(append(root, 3), 4)(input);
}

int main() {
  return use(5) == 12 ? 0 : 1;
}
