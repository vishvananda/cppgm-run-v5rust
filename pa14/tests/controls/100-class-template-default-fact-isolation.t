// expect exit 0
// Each concrete default is analyzed in its declaring parameter frame.
template<class T, class U = decltype(T() + 1)> struct Result;
using Small = Result<char>;
using Wide = Result<long>;
using Small = Result<char, int>;
using Wide = Result<long, long>;
template<class T, class U = T[sizeof(T)]> struct Array;
using IntArray = Array<int>;
using IntArray = Array<int, int[4]>;
template<class T> int width(Array<T, T[sizeof(T)]>*) { return sizeof(T); }
int main() { return width((IntArray*)0) - 4; }
