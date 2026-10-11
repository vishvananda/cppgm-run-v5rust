template<class T> struct Base { T x; };
struct Derived : Base<int> { int y; };
static_assert(sizeof(Derived) == sizeof(int) * 2, "");
