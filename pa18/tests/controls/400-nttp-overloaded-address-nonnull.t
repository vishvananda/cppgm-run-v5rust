// N3485 14.3.2p1: a call's non-null pointer is no named designation; expect reject
int value;
struct X {
  constexpr int* operator&() const { return &value; }
};
constexpr X x{};
template<int* P> struct A {};
A<&x> a;
