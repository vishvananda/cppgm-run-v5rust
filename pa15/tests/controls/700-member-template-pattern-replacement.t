// expect exit 0
// N3485 14.7.3p16-p18: a specialization replaces one instance's member template.
template<class T> struct Box {
  template<class U> int value(U) { return sizeof(T); }
  template<class... U> int count(U...) { return 1; }
  template<class U> int unused(U) { return U::missing; }
};
template<> template<class V> int Box<int>::value(V v) { return v + 7; }
template<> template<class... V> int Box<int>::count(V...) { return sizeof...(V); }
int main() {
  Box<int> a;
  Box<char> b;
  return a.value(1) == 8 && a.value(2) == 9 && b.value(3) == 1 &&
         a.count(1, 2L, '3') == 3 && b.count(1, 2L) == 1 ? 0 : 1;
}
