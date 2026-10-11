// VALIDATION: compile-pass

struct base {};
struct left : base {};
struct right : base {};
struct both : left, right {};

template<class T>
constexpr int size_of(const T &) { return sizeof(T); }

static_assert(size_of(base()) == sizeof(base), "");
static_assert(size_of(both()) == sizeof(both), "");

int main() { return 0; }
