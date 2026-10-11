struct M { M() = default; };
static_assert(noexcept(M()), "");
int main() { return noexcept(M()) ? 0 : 1; }
