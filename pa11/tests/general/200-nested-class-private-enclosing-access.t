class B { int n; public: struct D; };
struct B::D : B { int f() { return n = 0; } };
int main() { B::D d; return d.f(); }
