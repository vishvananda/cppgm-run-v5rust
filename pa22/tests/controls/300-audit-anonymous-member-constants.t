// expect exit 0
struct S { int a; union { int x; int y; }; };
struct A { long a; };
struct D : A, S {};
constexpr S saved = {1, {5}};
constexpr int S::*p = &S::x;
constexpr int D::*adjusted = p;
constexpr int S::*reversed = static_cast<int S::*>(adjusted);
static_assert(saved.*p == 5, "anonymous member designation");
static_assert(reversed == p, "anonymous member declaring class");
int main() { D d; d.x = 5; return d.*adjusted - 5; }
