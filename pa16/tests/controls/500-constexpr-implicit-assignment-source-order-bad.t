// N3485 5.19p2: implicit assignment reads its source before writing its target;
// expect reject
struct Object { int n; constexpr Object(int n) : n(n) {} };
Object first(3), second(7);
constexpr int assign() { first = second; return 0; }
constexpr int value = assign();
int main() { return value; }
