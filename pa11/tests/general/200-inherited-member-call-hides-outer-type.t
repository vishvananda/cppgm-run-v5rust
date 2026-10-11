enum class f {};
int value;
struct B { int* f() { return &value; } };
struct D : B { int* g() { return f(); } };
int main() { D d; return d.g() == &value ? 0 : 1; }
