// N3485 5.2.5p5, 11.2p6 (naming class), 10.2: d.L::v / L::v name the member of
// L's S subobject when S is a repeated base; expect exit 0
struct S { int v; };
struct L : S {};
struct R : S {};
struct D : L, R { int g() { L::v = 3; R::v = 4; return L::v * 10 + R::v; } };
int main() { D d; d.L::v = 1; d.R::v = 2; int a = d.L::v * 10 + d.R::v; return a == 12 && d.g() == 34 ? 0 : 1; }
