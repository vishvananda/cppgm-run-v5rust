// N3485 14.5.5.1p1, 14.7.1p1: an ambiguous partial-specialization match is an
// error only when S<int,int> is instantiated; naming it as a pointer type does
// not instantiate it; expect exit 0
template<class T, class U> struct S { static const int v = 0; };
template<class T> struct S<T, int> { static const int v = 1; };
template<class U> struct S<int, U> { static const int v = 2; };
S<int, int>* p = 0;
int main() { return p == 0 ? 0 : 1; }
