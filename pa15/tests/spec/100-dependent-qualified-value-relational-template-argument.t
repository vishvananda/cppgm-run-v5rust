template<bool, class, class> struct I;
template<class> struct R;
template<class A, class B> struct H {
  typedef typename I<R<A>::v < R<B>::v, B, A>::type type;
};
int main() {}
