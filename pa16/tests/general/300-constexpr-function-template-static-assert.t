template<class T>
constexpr T twice(T x) { return x + x; }

static_assert(twice<int>(3) == 6, "bad");

int main() { return 0; }
