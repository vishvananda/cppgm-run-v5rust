template<class T> struct X { typedef void F(T); F f; };
template<class T> void X<T>::f(T x) { if(x) return; }
int main() { X<int> x; x.f(0); }
