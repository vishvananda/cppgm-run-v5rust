// The core constant-expression query still rejects dynamic_cast; expect reject
// A static relocation probe must not change core constant-expression rules.
struct B { virtual ~B() {} };
struct D : B {};
B object;
constexpr D& reference = dynamic_cast<D&>(object);
