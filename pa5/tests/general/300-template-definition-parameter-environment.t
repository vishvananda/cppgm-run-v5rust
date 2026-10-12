// An out-of-class definition sees its own head, not the class's original head.
template<int T> struct C { void f(); };
template<class T> void C<T>::f() { T value; }
template<class T> struct D { void f(int); };
template<int T> void D<T>::f(int value = T < 2) { int x = T < 2; }
template<class T> struct E { struct Inner; };
template<int T> struct E<T>::Inner { int f() { return T < 2; } };
