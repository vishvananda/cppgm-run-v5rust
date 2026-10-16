// N3485 15.4p3, 8.3.6p6: exception agreement precedes the check for a
// new outside default, whose unknown name must remain unchecked; expect reject.
template<class T> struct A { int f(int) noexcept; };
template<class U> int A<U>::f(int x = missing) { return x; }
int main() { return 0; }
