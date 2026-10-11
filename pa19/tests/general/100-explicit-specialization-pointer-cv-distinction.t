template<class T>
struct tag;

template<>
struct tag<char* const> { static constexpr int value = 1; };

template<>
struct tag<const char*> { static constexpr int value = 2; };

static_assert(tag<char* const>::value == 1, "tag1");
static_assert(tag<const char*>::value == 2, "tag2");

int main() { return 0; }
