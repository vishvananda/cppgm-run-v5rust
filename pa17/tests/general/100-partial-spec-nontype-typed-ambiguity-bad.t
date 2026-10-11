// N3485 14.5.5.1p1, 14.5.5.2: S<IC<bool,false>> matches S<IC<T,0>> and
// S<IC<bool,b>>, neither more specialized: ambiguous, ill-formed; expect
// reject
template<class T, T v> struct IC {};
template<class X> struct S { static const int v = 0; };
template<class T> struct S<IC<T, 0> > { static const int v = 2; };
template<bool b> struct S<IC<bool, b> > { static const int v = 3; };
int main() { return S<IC<bool, false> >::v; }
