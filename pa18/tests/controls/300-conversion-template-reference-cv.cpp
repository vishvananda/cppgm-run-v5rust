// N3485 14.8.2.3p2, p4: for P = T& and target const int&, A is the referred
// type const int, so T = const int; expect exit 0
template<class T> struct is_const { static const int v = 0; };
template<class T> struct is_const<const T> { static const int v = 1; };
int x = 5, r = 9;
struct A { template<class T> operator T&() { r = is_const<T>::v; return x; } };
int main() { A a; const int& c = a; return &c == &x && r == 1 ? 0 : 1; }
