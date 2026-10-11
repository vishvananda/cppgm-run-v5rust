// N3485 15.4p2, 5.3.7p3: a call through a pointer whose declarator has a
// non-throwing exception-specification cannot throw, so noexcept(fp()) is
// true; expect exit 0
int f() noexcept { return 1; }
int (*fp)() noexcept = f;
int main() { static_assert(noexcept(fp()), ""); return 0; }
