// N3485 7.3.3p15: a derived member function template with the same name,
// parameter-type-list and template-head hides the using-declared base
// template; expect exit 0
struct B { template <class T> int f(T) { return 1; } };
struct D : B { using B::f; template <class T> int f(T) { return 2; } };
int main() { D d; return d.f(1) == 2 ? 0 : 1; }
