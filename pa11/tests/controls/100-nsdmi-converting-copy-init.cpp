// N3485 12.6.2p8/8.5p16: the brace-or-equal-initializer W2 c = W(1)
// copy-initializes c through the converting constructor W2(W) from a class
// prvalue; expect exit 0
struct W { int w; W(int v) : w(v) {} };
struct W2 { int w2; W2(W w) : w2(w.w) {} };
struct M { W2 c = W(1); };
int main() { M m; return m.c.w2 - 1; }
