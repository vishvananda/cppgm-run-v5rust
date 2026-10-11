template<bool B> struct R { static const bool value = B; };
struct X { int n; };
struct Y { Y(); };
template<class T> struct A: R<__is_aggregate(T)> {};
static_assert(A<X>::value, "");
static_assert(!A<Y>::value, "");
int main() {}
