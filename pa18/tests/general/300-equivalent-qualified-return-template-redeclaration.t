template<bool, class T> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };
namespace n {
template<class T>
typename enable_if<(sizeof(T) > 0), int>::type make(T&&);
template<class T, class... Args> long make(T&&, Args&&...);
template<class T>
typename ::enable_if<(sizeof(T) > 0), int>::type make(T&&) { return 0; }
}
int main() { return n::make(1); }
