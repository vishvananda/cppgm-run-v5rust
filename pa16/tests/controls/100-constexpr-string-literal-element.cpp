// N3485 5.19p2 (note: a string literal is an array of const objects
// initialized with constant expressions): a constexpr function reading s[0] of
// a string-literal argument is a constant expression; expect exit 0
typedef decltype(sizeof(0)) size_t;
constexpr size_t len(const char* s, size_t n) { return n + (s[0] == 'a'); }
static_assert(len("abc", 3) == 4, "");
int main() { return 0; }
