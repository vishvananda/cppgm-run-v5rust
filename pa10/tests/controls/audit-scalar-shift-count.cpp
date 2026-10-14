// expect exit 0
int main() {
    int x = 8;
    long count = 2;
    long wide = 8;
    unsigned small = 2;
    return ((x << count) != 32) + ((x >> count) != 2)
        + ((wide << small) != 32) + ((wide >> small) != 2);
}
