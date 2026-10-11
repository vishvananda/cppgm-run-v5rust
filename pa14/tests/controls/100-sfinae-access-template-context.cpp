// N3485 14.8.2p8, 11p6-p7: access in a substituted signature is checked in the
// template's context, not the caller's: private A::run fails f even when
// called from A's member; expect exit 0
class A { int run(); public: static int go(); };
template <class T> auto f(T* t) -> decltype(t->run(), char()) { return 0; }
long f(...) { return 0; }
int A::go() { return sizeof(f((A*)0)); }
int main() { return A::go() == sizeof(long) ? 0 : 1; }
