struct Base {
    int base;
    constexpr Base() : base(3) {}
};
struct Empty {};
struct Value : Base {
    const int first;
    Empty empty;
    int last;
    constexpr Value() : first(1), empty{}, last(2) {}
};
constexpr Value original{};
constexpr Value copied = original;
static_assert(original.base == 3 && copied.first == 1 && copied.last == 2, "values");
int main() { return copied.last - 2; }
