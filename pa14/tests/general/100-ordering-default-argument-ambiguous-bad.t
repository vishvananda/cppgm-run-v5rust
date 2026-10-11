// N3485 14.5.6.2p5 (example): parameters with default arguments not used in
// the call are not ordered, so f<int>(42) is ambiguous; must be rejected
template<class T, class U> struct A {};
template<class T, class U> int f(U, A<U, T>* p = 0) { return 1; }
template<class U> int f(U, A<U, U>* p = 0) { return 2; }
int main() { return f<int>(42); }
