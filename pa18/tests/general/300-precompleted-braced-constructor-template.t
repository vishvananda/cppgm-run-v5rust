template<int> struct A { template<class T> explicit A(const T&); };
template<int X, int Y> bool operator<(A<X>, A<Y>);
typedef A<2> B;
int main() { A<1> a(0); (void)(a < B{1}); }
