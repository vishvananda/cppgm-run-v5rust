static_assert(1L > 0, "");
static_assert(2147483648L > 0, "");
static_assert(0x7fffffffffffffffL > 0, "");
static_assert(-0x7fffffffffffffffL < 0, "");
static_assert(1000000000000000000L > 0, "");

int main() { return 0; }
