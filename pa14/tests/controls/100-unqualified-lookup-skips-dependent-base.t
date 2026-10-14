// N3485 14.6.2p3: unqualified lookup in a template member does not search the
// dependent base base<T>, so value* p declares a pointer to the global typedef
// int value; expect exit 0
typedef int value;
template<class T> struct base { static const int value = 3; };
template<class T> struct derived : base<T> { int f() { value* p = 0; return p == 0 ? 1 : 9; } };
int main() { derived<int> d; return d.f() == 1 ? 0 : 1; }
