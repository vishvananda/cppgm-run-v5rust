constexpr int one = 1;
constexpr int two = one + one;
static_assert(two == 2, "bad");
int main() { return 0; }
