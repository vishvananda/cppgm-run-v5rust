constexpr int choose(int x) { int y = x + 2; if (y > 4) return y; return 4; }
static_assert(choose(3) == 5, "bad");
int main() { return 0; }
