// N3485 14.6/2: a dependent qualified name without typename is not a type, so
// D<T>::x * q; is an expression statement and (D<T>::x) * q is a
// multiplication, not a cast; expect exit 0
template<class T> struct D { static const int x = 3; };
template<class T> int f(int q) { D<T>::x * q; return (D<T>::x) * q; }
int main() { return f<int>(5) == 15 ? 0 : 1; }
