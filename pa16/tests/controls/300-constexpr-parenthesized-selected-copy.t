// N3485 8.5 and 12.8: parenthesized constant initialization executes its selected copy.
// expect exit 0
struct Value {
    int n;
    constexpr Value(int n) : n(n) {}
    constexpr Value(const Value& other) : n(other.n + 1) {}
    template<class T> constexpr Value(const T&) : n(7) {}
};
constexpr Value first(3);
constexpr Value second(first);
constexpr Value third{second};
static_assert(second.n == 4 && third.n == 5, "non-template copy wins equal conversions");
int main() { return second.n == 4 && third.n == 5 ? 0 : 1; }
