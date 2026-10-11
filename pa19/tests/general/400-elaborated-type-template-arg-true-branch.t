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
struct Holder {
  If<B, struct PrivateNat, int>* p;
};

int main() {
  Holder<true> h;
  h.p = 0;
  return h.p == 0 ? 0 : 1;
}
