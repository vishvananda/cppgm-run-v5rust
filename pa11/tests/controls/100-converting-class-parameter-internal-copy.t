// N3485 12.8p31 and 13.3.3.1p6: the first constructor conversion owns its copy.
// expect exit 0
struct Immovable {
    int n;
    Immovable(int v) : n(v) {}
    Immovable(const Immovable&) = delete;
};
struct Box {
    int n;
    Box(Immovable value) : n(value.n) {}
};
int consume(Box value) { return value.n; }
struct Value {
    int n;
    Value(int v) : n(v) {}
    Value(const Value& other) : n(other.n + 1) {}
};
struct Wrapper {
    int n;
    Wrapper(Value value) : n(value.n) {}
};
int consume(Wrapper value) { return value.n; }
int main() {
    Value value(10);
    return consume(Immovable(7)) == 7 && consume(value) == 11 ? 0 : 1;
}
