// N3485 8.5.4p7: long->int after a constexpr conversion function whose
// constant value fits is not narrowing, so B{cl} selects B(int); expect exit 0
struct CL { constexpr operator long() const { return 9; } };
struct B { int v; B(int x) : v(x) {} };
constexpr CL cl{};
int main() { B b{cl}; return b.v == 9 ? 0 : 1; }
