// expect exit 0
int values[2];
int main() {
    int x = 0;
L:
    return x;
    const int &r = 3;
    volatile int &&v = 4;
    int (&a)[2] = values;
M:
    goto L;
}
