struct Value {
    int value;
    constexpr Value(int n) : value(n) {}
    constexpr operator int() const { return value; }
};
constexpr double floating() { return 1.0; }
int classes(int index) {
    constexpr Value values[3] = {1, 2, 3};
    return values[index].value;
}
int nested(int index) {
    constexpr Value values[2][2] = {{1, 2}, {3, 4}};
    return values[index][0].value;
}
double scalars(int index) {
    double values[2] = {floating(), 2.0};
    return values[index];
}
int converted(int index) {
    int values[2] = {Value{1}, 2};
    return values[index];
}
int main() { return classes(1) + nested(1) + int(scalars(0)) + converted(0) - 7; }
