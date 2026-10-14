// Itanium 5.1.4.6 + SysV 3.2.3: a this-adjusting thunk for a variadic member
// returning a 24-byte class (hidden result pointer, this second) must forward
// all arguments; expect exit 0
struct R { long a, b, c; };
struct C1 { long c1; virtual ~C1() {} };
struct C2 { virtual R make(int n, ...) { R r = { 0, 0, 0 }; return r; } virtual ~C2() {} };
struct E : C1, C2 { R make(int n, ...) { R r = { n, 0, 0 }; return r; } };
int main() { E e; C2& c = e; return c.make(3, 1L, 2.0).a == 3 ? 0 : 1; }
