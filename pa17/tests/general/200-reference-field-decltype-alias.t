template<class T> using I = T;
template<class T> struct A { T x; typedef I<decltype(x + x)> Y; };
template<class T> struct B { typedef typename A<T>::Y Y; };
typedef B<int>::Y Y;
int main() {}
