struct base {};
struct derived : virtual base {};

static_assert(!__is_empty(derived), "");

int main() {}
