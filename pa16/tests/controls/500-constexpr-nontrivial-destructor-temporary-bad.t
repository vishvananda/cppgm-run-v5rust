// N3485 5.19p2: constant initialization cannot create nonliteral temporaries;
// expect reject
struct Object { int n; constexpr Object(int x) : n(x) {} ~Object() {} };
constexpr int read(const Object& object) { return object.n; }
constexpr int value = read(Object(7));
int main() { return value; }
