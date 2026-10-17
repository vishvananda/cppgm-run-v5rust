// N3485 5.19p2: equal type and offset do not make an inactive named variant
// readable in a constant expression; expect reject
union U { int a; int b; };
constexpr int read() { U u = {3}; u.b = 7; return u.a; }
constexpr int n = read();
int main() { return n; }
