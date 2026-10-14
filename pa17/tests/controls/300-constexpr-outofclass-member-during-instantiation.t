// N3485 5.19/14.7.1: T<int>'s member typedef calls constexpr member template
// T<E>::g, defined out of class before T<int> is instantiated; the dependent
// call is a constant expression; expect exit 0
template<bool B> struct cond { typedef char type; };
template<> struct cond<false> { typedef int type; };
template<class E> struct T {
  template<class U> static constexpr int g(U u);
  typedef typename cond<g(E(1)) == 3>::type type2;
};
template<class E> template<class U> constexpr int T<E>::g(U u) { return u + 2; }
int main() { return sizeof(T<int>::type2) == 1 ? 0 : 1; }
