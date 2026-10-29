// N3485 5.19, 8.5 and 12.8: internal copies execute their retained conversion.
// expect exit 0
struct Value {
    int n;
    constexpr Value(int v) : n(v) {}
    constexpr Value(const Value& other) : n(other.n + 1) {}
};
struct Box {
    int n;
    constexpr Box(Value value) : n(value.n) {}
};
constexpr int consume(Box box) { return box.n; }
constexpr Value original(5);
static_assert(consume(original) == 6, "constructor parameter copy");
struct Proxy {
    constexpr Proxy() {}
    constexpr operator const Value&() const { return original; }
};
constexpr Proxy proxy;
constexpr Value result = proxy;
static_assert(result.n == 6, "conversion result copy");
struct Own {
    constexpr Own() {}
    constexpr operator Value() const { return Value(9); }
};
constexpr Value direct = Own();
static_assert(direct.n == 9, "own prvalue initializes in place");
int main() { return result.n == 6 && direct.n == 9 ? 0 : 1; }
