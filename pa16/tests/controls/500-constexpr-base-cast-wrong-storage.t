// expect reject
// N3485 5.2.9p2: a downcast requires an enclosing derived object.
struct Head { int head; };
struct Base { int value; constexpr Base() : value(7) {} };
struct Derived : Head, Base { int tail; };
constexpr int read(Base b) { return static_cast<Derived&>(b).tail; }
constexpr int invalid = read(Base());
