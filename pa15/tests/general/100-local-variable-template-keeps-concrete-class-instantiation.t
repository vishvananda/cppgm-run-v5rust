template<bool B> struct flag {};

namespace N {
  template<class T> struct Alloc {};

  template<class T> inline const bool flag_v = true;
  template<class T> inline const bool flag_v<T&> = true;
  template<class T> inline const bool flag_v<T&&> = true;
}

void f() {
  struct C { int x; };
  flag<N::flag_v<N::Alloc<C>>> x;
}
