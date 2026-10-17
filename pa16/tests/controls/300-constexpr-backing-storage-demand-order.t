// N3485 3.6.2, 12.2p5 and 14.7.1: backing images demand template static storage.
// expect exit 0
template<class T> struct Storage { static T value; };
template<class T> T Storage<T>::value;
struct First { int n; First() : n(3) {} };
struct Second { int n; Second() : n(5) {} };
struct Pointer { First* p; constexpr Pointer(First* p) : p(p) {} };
struct Binding { Second* const& p; };
const Pointer& pointer = Pointer(&Storage<First>::value);
constexpr Binding binding = { &Storage<Second>::value };
int main() { return pointer.p->n == 3 && binding.p->n == 5 ? 0 : 1; }
