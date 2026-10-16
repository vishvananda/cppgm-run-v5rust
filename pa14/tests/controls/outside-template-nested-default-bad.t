// N3485 8.3.6p6: a nested class is also a member pattern, even when its
// function is never instantiated; expect reject.
template<class T> struct A { struct B { int f(int); }; };
template<class U> int A<U>::B::f(int x = 0) { return x; }
int main() { return 0; }
