// expect exit 0
int global[3][3] = {{1, 2}, {3, 4}, {5}};
int read(int (*p)[3]) { return p[1][1]; }
int calls;
int next() { return ++calls; }
int main() {
    int values[2][2] = {{next(), next()}, {next()}};
    volatile unsigned char bytes[2][2] = {{1, 2}, {3}};
    int *local[2] = {values[0]};
    const char *labels[2][2] = {{"a", "b"}, {"c"}};
    int (*p)[3] = global;
    p += 2;
    --p;
    return calls != 3 || values[1][0] != 3 || values[1][1] != 0
        || bytes[1][0] != 3 || bytes[1][1] != 0 || p - global != 1 || read(global) != 4
        || local[0][1] != 2 || local[1] != nullptr || labels[1][0][0] != 'c' || labels[1][1] != nullptr;
}
