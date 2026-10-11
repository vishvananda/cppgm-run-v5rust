template<class T, unsigned Mode>
struct outer {
  template<class D>
  struct inner {
    static constexpr decltype(D::pointer) pointer = D::pointer;
  };
};

template<class T, unsigned Mode>
template<class D>
constexpr decltype(D::pointer) outer<T, Mode>::inner<D>::pointer;

struct descriptor {
  static constexpr int pointer = 7;
};

int main() {
  return outer<int, 1>::inner<descriptor>::pointer == 7 ? 0 : 1;
}
