// N3485 8.3.6p6: defaults belong to the class-template declaration;
// an unused out-of-class constructor cannot add one; expect reject.
template<class T> struct A { A(int); };
template<class U> A<U>::A(int x = 0) {}
int main() { return 0; }
