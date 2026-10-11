template<bool B, class T, class F>
struct IfC {
  typedef T type;
};

template<class T, class F>
struct IfC<false, T, F> {
  typedef F type;
};

template<bool B, class T, class F>
using If = typename IfC<B, T, F>::type;

template<bool B>
struct Iter {
  Iter& operator=(const If<B, struct PrivateNat, Iter>&) {
    return *this;
  }
};

int main() {
  Iter<false> a;
  Iter<false> b;
  a = b;
  return 0;
}
