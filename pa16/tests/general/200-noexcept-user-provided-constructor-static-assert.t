struct M { M() {} };
static_assert(!noexcept(M()), "");
int main() { return noexcept(M()) ? 1 : 0; }
