template <class T>
struct Box {};

template <class T>
struct Pick {
  static int value() { return 0; }
};

template <class K, class V>
struct PairLike {};

template <class K, class V>
struct Pick<PairLike<K, V> > {
  static int value() { return 1; }
};

struct S {
  enum E { A, B };
};

int main() {
  return Pick<PairLike<int, Box<S::E> > >::value() == 1 ? 0 : 1;
}
