// N3485 14.5.2p3: a member function template shall not be virtual; expect
// reject
struct A { template<class C> virtual void g(C) {} };
int main() { return 0; }
