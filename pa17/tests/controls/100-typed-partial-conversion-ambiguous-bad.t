// expect reject
// Reversing declaration order cannot rank crossing value/type constraints.
template<class T, T Value> struct constant {};
template<class T> struct select {};
template<short Value> struct select<constant<short, Value>> {};
template<class T> struct select<constant<T, 0>> {};
int main() { return sizeof(select<constant<short, 0>>); }
