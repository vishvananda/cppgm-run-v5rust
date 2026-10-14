// N3485 14.5.6.2p3 (its example): a member template is ordered with an
// inserted first parameter B<A>&, so B<A>::operator*(R&) is more specialized
// than operator*(T&, R&); expect exit 0
struct A {};
template<class T> struct B { template<class R> int operator*(R&) { return 1; } };
template<class T, class R> int operator*(T&, R&) { return 2; }
int main() { A a; B<A> b; return b * a == 1 ? 0 : 1; }
