// N3485 focus: 5.2.5 [expr.ref], 5.16 [expr.cond]
struct X { int value; constexpr int const& get() const { return value; } };

constexpr int const* get_if(X const* p, bool active) {
  return p && active ? &p->get() : 0;
}

constexpr X x = {1};
static_assert(get_if(&x, false) == nullptr, "inactive");
static_assert(get_if(&x, true) == &x.value, "active");
int main() {}
