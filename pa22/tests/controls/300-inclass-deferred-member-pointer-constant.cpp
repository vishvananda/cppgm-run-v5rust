// A static data member's in-class initializer converting a pointer to member
// to the class being defined waits for its completion (its base offsets,
// 4.11p2); a constant expression in the class that reads it (a later member's
// initializer, a static assertion) uses its value with the provisional base
// offsets, which compare as the final ones will (5.10p2). expect exit 0
struct B { int m; int n; };
struct D : B {
  static constexpr int D::* p = &B::m;
  static constexpr int D::* q = &B::n;
  static constexpr bool pq = p == q;
  static constexpr bool pp = p == p;
  static_assert(!pq && pp, "");
  int k;
};
constexpr int D::* D::p;
constexpr int D::* D::q;
int main() { D d; d.m = 2; d.n = 3; return d.*D::p == 2 && d.*D::q == 3 && !D::pq && D::pp ? 0 : 1; }
