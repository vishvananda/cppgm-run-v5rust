// expect exit 0
template<class T> struct A { int f() { return 1; } };
template<> int A<int>::f();
template<> int A<int>::f() = delete;
int main() { return 0; }
