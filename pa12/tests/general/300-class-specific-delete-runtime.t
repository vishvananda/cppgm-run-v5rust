int calls;
struct U { static void operator delete(void *) noexcept { ++calls; } };
struct S { static void operator delete(void *, decltype(sizeof(0))) noexcept { ++calls; } };
int main() { U *u = new U; delete u; S *s = new S; delete s; return calls == 2 ? 0 : 1; }
