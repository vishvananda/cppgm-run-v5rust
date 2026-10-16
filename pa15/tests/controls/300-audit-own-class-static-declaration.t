// expect exit 0
template<class T> struct A { static int n; };
template<> struct A<int> { static int n; };
template<> int A<int>::n;
int main() { return 0; }
