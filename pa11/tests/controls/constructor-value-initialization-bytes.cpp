// expect exit 0
struct Values {
    volatile unsigned long wide;
    unsigned char small;
    int marker = 3;
};
struct UnionHolder {
    union Part { int number; char byte; } part;
    int marker = 5;
};
int main() {
    Values values{};
    UnionHolder holder{};
    return values.wide == 0 && values.small == 0 && values.marker == 3 &&
           holder.part.number == 0 && holder.marker == 5 ? 0 : 1;
}
