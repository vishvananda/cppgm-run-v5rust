// N3485 8.3.6p9, 14.7.1p13: a function template default argument (k = v) is
// looked up in the scope of the declaration that introduced it (namespace M),
// even when the definition is the qualified template<class T> int M::w(...);
// expect exit 0
namespace M { int v = 1; template<class T> int w(T, int k = v); }
template<class T> int M::w(T, int k) { return k; }
int main() { return M::w(3) == 1 ? 0 : 1; }
