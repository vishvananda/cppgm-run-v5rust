// N3485 12.8p28: implicit assignment executes the selected subobject actions;
// expect exit 0
struct Base { int n; constexpr Base(int n) : n(n) {} };
struct Member {
    mutable int n;
    constexpr Member(int n) : n(n) {}
    constexpr const Member& operator=(const Member& other) const { n = other.n + 1; return *this; }
};
struct Object : Base {
    Member member;
    int values[2];
    constexpr Object(int n) : Base(n), member(n), values{n, n + 1} {}
};
constexpr int assign() {
    Object a(3), b(7);
    const Object& result = (a = b);
    return result.n * 1000 + result.member.n * 100 + result.values[1] * 10 + b.member.n;
}
static_assert(assign() == 7887, "base, nontrivial member, array and returned reference");
int main() { return assign() == 7887 ? 0 : 1; }
