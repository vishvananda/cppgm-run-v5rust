// N3485 5.3.7: inspect the selected overload without emission demand.
inline int choose(int) noexcept { return 1; }
inline int choose(long) { return 2; }
int main() { return noexcept(choose(1)) + noexcept(choose(1L)); }
