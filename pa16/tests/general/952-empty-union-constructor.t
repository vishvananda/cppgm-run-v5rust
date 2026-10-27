struct Empty {};
union Value {
    Empty empty;
    int number;
    constexpr Value() : empty() {}
};
constexpr Value value{};
int main() { return 0; }
