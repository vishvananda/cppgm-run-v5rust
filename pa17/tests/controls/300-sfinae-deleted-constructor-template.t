// N3485 8.4.3p2, 13.3.3, 14.8.2p8: A(declval<double>()) selects the deleted
// constructor template A(T) (exact match), so the expression is invalid and
// deduction fails; expect exit 0
template<class T> T&& declval();
struct A { template<class T> A(T) = delete; A(int) {} };
template<class T> auto construct(int) -> decltype(A(declval<T>()), char()) { return 0; }
template<class T> long construct(...) { return 0; }
int main() { return sizeof(construct<double>(0)) == sizeof(long) ? 0 : 1; }
