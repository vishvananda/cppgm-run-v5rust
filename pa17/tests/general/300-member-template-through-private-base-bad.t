// N3485 11.2p4-p5: a public member template of a private base named through
// the derived class is inaccessible outside it; expect reject
struct B { template<class T> static int f() { return 1; } };
struct D : private B { };
int main() { return D::f<int>(); }
