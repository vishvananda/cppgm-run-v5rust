// N3485 3.6.2p2: declaration storage persists beyond constant initialization;
// expect exit 0
struct Object { int n; constexpr Object(int x) : n(x) {} ~Object() {} };
extern Object object;
int early = object.n;
Object object(7);
int main() { return early == 7 && object.n == 7 ? 0 : 1; }
