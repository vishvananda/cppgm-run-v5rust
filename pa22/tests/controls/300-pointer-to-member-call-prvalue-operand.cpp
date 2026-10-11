// N3485 5.5p2-p6, 5.2.2p1: (s.*pm())(2) calls through a
// pointer-to-member-function prvalue returned by a call; expect exit 0
struct S { int v; int get(int a) const { return v + a; } };
int (S::*pm())(int) const { return &S::get; }
int main() { S s = { 1 }; return (s.*pm())(2) == 3 ? 0 : 1; }
