// N3485 6.6.3p2/12.8p32: return t; for a static (non-automatic) lvalue t
// copy-initializes the result with T3(T3&) (non-const copy constructor);
// expect exit 0
struct T3 { int a, b; T3(int a) : a(a), b(a + 1) {} T3(T3&) = default; };
static T3 r3(int k) { static T3 t(0); t.a = k; return t; }
int main() { return r3(4).a - 4; }
