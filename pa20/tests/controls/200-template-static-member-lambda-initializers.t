// expect exit 0
// The lambda body is analyzed where the demanded initializer is instantiated,
// including nested closures. Repeated member uses share one closure and static.
template<class T> struct width { static int (*get)(); };
template<class T> int (*width<T>::get)() = [] {
  static int calls = 0;
  auto size = [] { return int(sizeof(T)); };
  return calls += size();
};
int first() { return width<int>::get(); }
int main() {
  int (*p)() = width<int>::get;
  if (first() != 4 || width<char>::get() != 1) return 1;
  return p() == 8 && width<char>::get() == 2 ? 0 : 2;
}
