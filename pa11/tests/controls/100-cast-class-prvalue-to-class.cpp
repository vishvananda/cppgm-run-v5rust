// N3485 5.4p4, 5.2.9p4: an explicit cast (T)e / static_cast<T>(e) of a class
// prvalue to its own (or const) class type is direct-initialization of a
// temporary; expect exit 0
struct D { int m; D() : m(7) {} ~D() {} };
int use(const D& d) { return d.m; }
int main() { return use((D)D()) + use(static_cast<const D>(D())) == 14 ? 0 : 1; }
