namespace n { typedef int x; }
struct B { static bool const v = false; };
template<class> struct H : B {};
template<bool> struct S { typedef int t; };
template<class T> struct M {
  static bool const v = H<n::x>::v;
  typedef typename S<v>::t n;
};
