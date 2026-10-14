// N3485 8.4.3p2, 5.2.3p2, 14.8.2p8: decltype(T()) for T with a deleted default
// constructor is invalid, so make<Del> fails deduction; expect exit 0
struct Del { Del() = delete; };
template<class T, class = decltype(T())> char make(int);
template<class> long make(...);
int main() { return sizeof(make<Del>(0)) == sizeof(long) ? 0 : 1; }
