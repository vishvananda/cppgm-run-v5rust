// expect exit 0
int calls;
int next() { return ++calls; }
int main() {
    int dynamic[4] = {next(), next()};
    volatile int observed[3] = {7, 8};
    int other[4] = {4, 5};
    (calls ? dynamic : other)[2] = 9;
    return calls != 2 || dynamic[0] != 1 || dynamic[1] != 2 || dynamic[2] != 9
        || dynamic[3] != 0 || observed[0] != 7 || observed[2] != 0 || other[0] != 4;
}
