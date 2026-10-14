// expect exit 0
int qualified(const int x, volatile int y, int *const p) {
    return x + y + *p;
}
int indirect(const int x) {
    return x;
}
int uncalled(volatile int x) {
    return x;
}
int main() {
    int z = 1;
    int (*p)(const int) = indirect;
    return (qualified(1, 2, &z) != 4) + (p(2) != 2);
}
