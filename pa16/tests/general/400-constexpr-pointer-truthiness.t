constexpr const char *p = "x";
static_assert(p, "");
static_assert((p == nullptr) == false, "");
char a[3];
constexpr const char *q = a;
static_assert(q, "");
static_assert((q == nullptr) == false, "");

int main() { return 0; }
