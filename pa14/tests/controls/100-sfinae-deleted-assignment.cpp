// N3485 14.8.2p8/8.4.3p2: selecting a deleted copy assignment inside decltype
// during deduction is a substitution failure, not a hard error; expect exit 0
template<class T> T&& declval();
struct B { B& operator=(const B&) = delete; };
template<class T> auto fa(T*) -> decltype(declval<T&>() = declval<T&>(), 1) { return 1; }
template<class T> long fa(...) { return 2; }
int main() { return sizeof(fa<B>(0)) == sizeof(long) ? 0 : 1; }
