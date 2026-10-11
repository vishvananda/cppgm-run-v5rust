template<int N> struct X {};
template<int A, int B = 1> X<A + B> f(X<A>, X<B>);
void check(X<5>);
int main() { check(f(X<1>(), X<4>())); }
