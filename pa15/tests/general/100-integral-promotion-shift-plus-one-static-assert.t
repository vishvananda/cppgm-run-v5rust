static_assert(((1LL << (sizeof(long long) * 8 - 1)) + 1) < 0, "bad");
int main() { return 0; }
