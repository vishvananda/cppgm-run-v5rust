constexpr int twice(int x) { return x + x; }
int f(int x = twice(3)) { return x; }
int main() { return f(); }
