// expect exit 0
struct B { int x; };
struct C { int y; };
struct D : B, C {};
constexpr int B::*p = &B::x;
constexpr int D::*q = &C::y;
static_assert(p != q && q != p, "compare in the recorded derived owner");
static_assert(nullptr != q && q != nullptr, "null comparison conversion");
int main() { return 0; }
