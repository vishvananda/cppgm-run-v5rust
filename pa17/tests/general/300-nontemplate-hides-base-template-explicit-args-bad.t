// N3485 10.2, 14.2p3: D::f (a non-template) hides B::f template, so d.f<char>
// is not a template-id: ill-formed; expect reject
struct B { template<class T> int f(T) { return 1; } };
struct D : B { int f(int) { return 2; } };
int main() { D d; return d.f<char>(1); }
