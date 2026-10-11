// N3485 10p2 (class-or-decltype shall denote a class type): decltype((k)) is
// K1&, not a class type; must be rejected
struct K1 { int a; };
K1 k;
struct D : decltype((k)) {};
int main() { return 0; }
