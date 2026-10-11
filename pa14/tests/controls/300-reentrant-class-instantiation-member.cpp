// N3485 9.2p2, 3.4.3.1, 14.8.2: C<int> instantiated from f's return type;
// inside it decltype(f(T())) sees the earlier C<int>::type, so f<int> (long)
// is chosen, not f(...); expect exit 0
template<class T> struct C;
template<class T> typename C<T>::type f(T);
char f(...);
template<class T> struct C { typedef long type; decltype(f(T())) m() { return 7; } };
template<class T> typename C<T>::type f(T t) { return t; }
int main() { long r = f(1); C<int> c; return sizeof(c.m()) == sizeof(long) && r == 1 ? 0 : 1; }
