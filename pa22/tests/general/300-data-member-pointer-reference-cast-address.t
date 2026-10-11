struct B { int x; };
struct M : B {};
struct O { M m; };
inline B *addr(B &b) { return &b; }
template<M O::*P> B *get(O &o) { return addr(static_cast<B &>(static_cast<M &>(o.*P))); }
int main() { O o = {}; return get<&O::m>(o) != &o.m; }
