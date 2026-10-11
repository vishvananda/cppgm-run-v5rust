// VALIDATION: compile-fail
template<void (*)()> struct U {};
template<class T> struct B { static void f() { T::bad(); } typedef U<&B::f> x; };
template<class T> struct D : B<T> { typedef int type; };
template<class T> typename D<T>::type g(T *, long *);
template<class T> void g(T *, int) {}
int main() { g((int *)0, 0); }
