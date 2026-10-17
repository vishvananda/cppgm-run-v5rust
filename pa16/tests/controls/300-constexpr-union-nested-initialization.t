// N3485 7.1.5p4, 9.5p5, 8.5.1p15: nested variant construction and empty
// lists initialize the selected anonymous member object; expect exit 0
struct Pair {
    int x, y;
    constexpr Pair(int n) : x(n), y(n + 1) {}
};
union Selected {
    Pair a;
    Pair b;
    constexpr Selected(int n) : b(n) {}
};
union Inner { int n; double d; constexpr Inner() : d(2.5) {} };
struct Holder { Inner u; int n; constexpr Holder() : u(), n(7) {} };
union Outer { int n; Holder h; constexpr Outer() : h() {} };
union Anonymous { int n; union { int x; long y = 9; }; };
constexpr Selected selected(3);
constexpr Outer outer;
constexpr Anonymous anonymous{};
Anonymous ordinary{};
static_assert(selected.b.y == 4 && outer.h.u.d == 2.5, "constructed variants");
static_assert(anonymous.y == 9, "anonymous default initializer");
int main() {
    return selected.b.x == 3 && outer.h.n == 7 && ordinary.y == 9 ? 0 : 1;
}
