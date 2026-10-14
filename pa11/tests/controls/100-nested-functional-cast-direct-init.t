// N3485 6.8, 8.2p1: V v(O(A(5))); cannot be a function declaration (5 is not a
// declarator), so it is direct-initialization from a nested functional cast;
// expect exit 0
struct A { A(int) {} };
struct O { O(A) {} };
struct V { V(O) {} int x = 1; };
int main() { V v(O(A(5))); return v.x == 1 ? 0 : 1; }
