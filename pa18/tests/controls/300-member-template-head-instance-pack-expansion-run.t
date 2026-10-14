// N3485 14.5.3p6: an instance's pack expanded in its member declarations by a
// pattern that names it alone (`list<E>...`, `E*...`) or with a member
// template's parameter (`list<E, U>...`). expect exit 134
template<class... Ps> struct list { static const int n = sizeof...(Ps); };
template<bool B, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };
template<class... A> using L = typename enable_if<(list<A...>::n > 0), int>::type;
template<class... E> struct tup {
  template<class U> static int h(list<list<E, U>...>) { return list<list<E, U>...>::n; }
  template<class... Us, L<list<E, Us...>...> = 0> static int g(Us...) { return 2; }
  typedef list<list<E>...> lists;
  typedef list<E*...> ptrs;
};
int main() {
  int r = tup<int, char, long>::h<int>(list<list<int, int>, list<char, int>, list<long, int> >()) * 10;
  r += tup<int, char>::lists::n * 50 + tup<int, char>::ptrs::n + tup<int>::g(1, 2);
  return r;
}
