// N3485 15.4p1/14.5.3p4: a dynamic-exception-specification may contain a pack
// expansion throw(Ts...); expect exit 0
struct A {};
template<class... Ts> struct Y { int f(int k) throw(Ts...) { return k; } };
int main() { Y<A, int> y; return y.f(0); }
