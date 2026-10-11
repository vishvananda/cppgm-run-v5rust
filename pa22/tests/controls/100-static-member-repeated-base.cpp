// N3485 10.2p10 (note) / 10.2p6: a static member found through several
// subobjects of the same base class is not ambiguous; expect exit 0
struct S { static int s; };
int S::s = 5;
struct L : S {};
struct R : S {};
struct D : L, R {};
int main() { D d; return d.s == 5 && D::s == 5 ? 0 : 1; }
