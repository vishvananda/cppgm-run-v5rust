// N3485 8.3.6p6: a static member's unused outside definition cannot add
// a default argument; expect reject.
template<class T> struct A { static int f(int); };
template<class U> int A<U>::f(int x = sizeof(U)) { return x; }
int main() { return 0; }
