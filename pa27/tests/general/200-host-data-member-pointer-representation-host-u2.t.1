// Itanium ABI 2.3: a pointer to data member is the member's offset (null = -1)
// as a ptrdiff_t; cppgm++ and host objects exchanging one must agree;
// expect exit 0 under every mix of units built by cppgm++ and by the host
// compiler
struct S { int a; int b; };
typedef int S::* DM;
int read(S* o, DM d);
bool is_null(DM d);
DM get_b();
int main() { S s = { 1, 2 }; DM n = 0; return read(&s, &S::b) == 2 && is_null(n) && !is_null(get_b()) && s.*get_b() == 2 ? 0 : 1; }
