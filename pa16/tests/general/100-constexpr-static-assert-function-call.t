constexpr int twice(int x) { return x + x; }
static_assert(twice(3) == 6, "bad");
int main() { return 0; }
