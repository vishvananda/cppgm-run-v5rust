// N3485 14.1p4, 8.3.3: a member template of a class template with a non-type
// parameter of type int Holder::* (injected-class-name as the owner), used as
// get<&Holder::a>(); expect exit 0
template<class T> struct Holder { int a, b; Holder() : a(3), b(4) {} template<int Holder::*P> int get() const { return this->*P; } int both() const { return get<&Holder::a>() * 10 + get<&Holder::b>(); } };
int main() { Holder<int> h; return h.both() == 34 ? 0 : 1; }
