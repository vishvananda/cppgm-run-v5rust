template<class T> struct A { static const int x = 1; };
template<class T> int f() { return (A<T>::x) + 1; }
int main() { return f<int>() - 2; }
