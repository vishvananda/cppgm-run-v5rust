// N3485 14.6p10: non-dependent template arguments of a function template's
// signature, evaluated where it is defined (a class member's constexpr call,
// an object's address): the template is still deduced and called.
// expect exit 3
template<bool B, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };
struct K { static constexpr bool yes() { return true; } };
int x;
template<int* P> struct A { static int* get() { return P; } };
template<class T, typename enable_if<K::yes(), int>::type = 0> int f(T t, A<&x>* = 0) { return t + (A<&x>::get() == &x); }
int main() { return f(2); }
