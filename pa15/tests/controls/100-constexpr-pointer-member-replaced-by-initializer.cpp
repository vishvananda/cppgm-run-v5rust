// N3485 5.1.1/8, 9.4.2/3: the id-expression N::str names the static data
// member (an lvalue of type const char* const), not its initializer "nstr";
// &N::str is a const char* const*; expect exit 0
struct N { static constexpr const char* str = "nstr"; };
constexpr const char* N::str;
int main() { const char* const* q = &N::str; return *q == N::str && (*q)[0] == 'n' ? 0 : 1; }
