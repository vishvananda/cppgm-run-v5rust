// N3485 14.5.3p6, 14.7.1p1: in tup<>'s member template the expansion
// `list<E>...` of the instance's pack E is empty, so the parameter's type L<>
// depends on nothing and enable_if<false>::type is the instance's error, not a
// deduction failure of g.
template<bool B, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };
template<class... Ps> struct list { static const int n = sizeof...(Ps); };
template<class... A> using L = typename enable_if<(list<A...>::n > 0), bool>::type;
template<class... E> struct tup {
  template<class... Us, L<list<E, Us...>...> = false> static int g(Us...) { return 1; }
  static int g(...) { return 0; }
};
int main() { return tup<>::g(1); }
