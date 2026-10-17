// N3485 5.19p2: implicit assignment cannot modify storage outside evaluation;
// expect reject
struct Object { int n; constexpr Object(int n) : n(n) {} };
Object global(3);
constexpr int assign() { Object local(7); global = local; return global.n; }
constexpr int value = assign();
int main() { return value; }
