// N3485 14.3.2p1: overloaded & is a call, not an object designation; expect exit 0
struct Member {
  constexpr int* operator&() const { return nullptr; }
};
struct Free {};
constexpr int* operator&(Free) { return nullptr; }
constexpr Member member{};
constexpr Free object{};
template<int* P> struct A {};
template<class T> struct Same { static const bool value = false; };
template<int* P> struct Same<A<P>> { static const bool value = P == nullptr; };
static_assert(Same<A<&member>>::value, "member call returns null");
static_assert(Same<A<&object>>::value, "free call returns null");
A<&member> a;
A<&object> b;
int main() { return sizeof(a) + sizeof(b) - 2; }
