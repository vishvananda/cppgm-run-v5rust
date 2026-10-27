union Value {
    struct { int first = 1; int second; };
    int number;
    constexpr Value() = default;
};
int main() { return 0; }
