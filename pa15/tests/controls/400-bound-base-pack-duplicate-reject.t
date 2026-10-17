// Canonical duplicate bases remain invalid after expansion. expect reject
struct Base {};
template<class... T> struct Derived : Base, T... {};
Derived<Base> value;
