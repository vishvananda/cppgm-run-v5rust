// N3485 14.8.2.4p11 (note), 14.8.2.5p5: f(T, T) is more specialized than f(T,
// typename id<T>::type) (non-deduced P succeeds); expect exit 0
template<class T> struct id { typedef T type; };
template<class T> int f(T, typename id<T>::type) { return 1; }
template<class T> int f(T, T) { return 2; }
int main() { return f(1, 1) == 2 ? 0 : 1; }
