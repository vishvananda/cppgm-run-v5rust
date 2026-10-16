// Constant template initializers do not demand address-target layout; expect exit 0
struct Incomplete;
template<class T> int value = 1;
template<> Incomplete value<int>;
template<class T> constexpr Incomplete* pointer = &value<int>;
static_assert(pointer<char> != nullptr, "constant address needs no storage");
template<class T> constexpr bool present = (&value<int> != nullptr);
int main() { return present<long> ? 0 : 1; }
