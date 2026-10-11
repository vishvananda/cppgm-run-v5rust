// N3485 5.2.3p1, 8.5p16, 8.5.4p3: P({4, 5}) direct-initializes P via its
// implicit copy/move constructor whose parameter is list-initialized;
// expect exit 0
struct P { int x, y; };
int main() { P p = P({4, 5}); return p.x + p.y == 9 ? 0 : 1; }
