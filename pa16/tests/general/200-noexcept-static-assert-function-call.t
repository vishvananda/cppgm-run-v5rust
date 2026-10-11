void f() noexcept {}
void g() {}
static_assert(noexcept(f()), "");
static_assert(!noexcept(g()), "");
int main() { return 0; }
