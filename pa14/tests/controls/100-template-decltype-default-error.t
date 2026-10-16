// N3485 14.8.2p8, 14.7.1p12: a default's required instantiation is outside
// a decltype substitution's immediate context; it cannot discard the candidate; expect reject
template<class T> int target(T, int value = T::missing);
template<class T> auto choose(T x) -> decltype(target(x));
long choose(...);
int main() { return sizeof(choose(1)); }
