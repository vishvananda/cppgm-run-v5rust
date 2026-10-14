// N3485 14.1p4 (std::nullptr_t is a permitted non-type parameter type),
// 14.3.2p1; expect exit 0
typedef decltype(nullptr) nullptr_t;
template<nullptr_t N> struct Z { static bool null() { return N == nullptr; } };
int main() { return Z<nullptr>::null() ? 0 : 1; }
