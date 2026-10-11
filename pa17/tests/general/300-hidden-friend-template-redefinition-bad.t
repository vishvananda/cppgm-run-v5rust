// N3485 14.5.4p4, 3.2p1: A<int> and A<char> both define friend template g(U):
// redefinition; expect reject
template<class T> struct A { template<class U> friend int g(U) { return 0; } };
A<int> a;
A<char> b;
int main() { return 0; }
