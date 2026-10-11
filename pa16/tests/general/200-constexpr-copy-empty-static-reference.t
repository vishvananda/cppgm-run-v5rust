struct E {};
constexpr E source{};
constexpr const E& get() { return source; }
constexpr int consume(E) { return 1; }

static_assert(consume(static_cast<E>(get())) == 1, "");
int main() { return 0; }
