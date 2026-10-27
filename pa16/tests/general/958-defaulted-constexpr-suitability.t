struct Value {
    int number = 7;
    constexpr Value() = default;
    constexpr Value(const Value &) = default;
};
constexpr Value original{};
constexpr Value copied = original;
static_assert(copied.number == 7, "defaulted members");
template<class T> struct Deferred {
    T number;
    constexpr Deferred() = default;
};
Deferred<int> ordinary;
int main() { return copied.number - 7; }
