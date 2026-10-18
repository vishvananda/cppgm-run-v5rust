// expect exit 0
// Each specialization owns its initializer closure and local static. Direct
// and parenthesized template-id calls select the same variable as a value use.
template<class T> int (*width)() = [] {
  static int calls = 0;
  return calls += int(sizeof(T));
};
template<class T> int invoke() { return width<T>(); }
template<class T> int (*unused)() = [] { return T::missing; };
template<class T> struct member_width { static int (*get)(); };
template<class T> int (*member_width<T>::get)() = [] {
  static int calls = 0;
  return calls += int(sizeof(T));
};
int main() {
  int (*p)() = width<int>;
  if (member_width<int>::get() != 4) return 4;
  if (invoke<int>() != 4 || (width<char>)() != 1) return 1;
  if (member_width<char>::get() != 1) return 5;
  if (p() != 8 || width<int>() != 12) return 2;
  return invoke<char>() == 2 && member_width<int>::get() == 8 ? 0 : 3;
}
