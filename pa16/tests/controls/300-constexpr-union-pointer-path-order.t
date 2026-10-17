// N3485 5.19p2, 9.5p1: pointer access follows active union storage, then
// declaration order when it starts a member's lifetime; expect exit 0
union Inner {
    long other;
    int first;
    int second;
    constexpr Inner() : second(5) {}
};
struct Branch {
    Inner inner;
    int marker;
    constexpr Branch() : inner(), marker(9) {}
};
union Outer {
    int plain;
    Branch left;
    Branch right;
    constexpr Outer() : right() {}
};
constexpr int active_path() {
    Outer object;
    int* pointer = &object.left.inner.first;
    *pointer += 2;
    return object.right.inner.second + object.right.marker;
}
union Fallback {
    long other;
    Inner first;
    Inner second;
    constexpr Fallback() : other(1) {}
};
constexpr int declaration_order() {
    Fallback object;
    int* pointer = &object.second.second;
    *pointer = 11;
    return object.first.first;
}
static_assert(active_path() == 16, "outer and inner active paths");
static_assert(declaration_order() == 11, "first compatible inactive path");
int main() { return active_path() == 16 && declaration_order() == 11 ? 0 : 1; }
