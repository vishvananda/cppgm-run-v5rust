// N3485 13.5.8/5.1.1p8: a literal operator may be named by a qualified-id
// N::operator"" _n and called/addressed like any function; expect exit 0
namespace N { unsigned long long operator"" _n(unsigned long long x) { return x + 1; } }
int main() { return N::operator"" _n(4) == 5 ? 0 : 1; }
