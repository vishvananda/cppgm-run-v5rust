// Nested regions restore angle context; definitions and extern declarations differ.
template<int N> struct Box {};
template<class T> int get();
template int get<int>();
extern template int get<long>();
template<class... T> void f() {
  Box<(1 > 2)> comparison;
  Box<(6 >> 1)> shift;
  Box<static_cast<int>(x)> cast;
  Box<(c ? get<int>() > 2 : 3)> conditional;
  call(args...);
  int count = sizeof...(T);
}
