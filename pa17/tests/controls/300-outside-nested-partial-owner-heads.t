// N3485 14.5.5.3: the third head's parameters are unavailable while the
// outer heads discover the qualified definition's owner; expect exit 0
template<class A> struct Outer {
  template<class B> struct Middle { template<class C> struct Inner; };
};
template<class A> template<class B> template<class C>
struct Outer<A>::Middle<B>::Inner<C*> { struct Member; };
template<class P> template<class Q> template<class R>
struct Outer<P>::Middle<Q>::Inner<R*>::Member {
  P first;
  Q second;
  R third;
};
static_assert(sizeof(Outer<int>::Middle<char>::Inner<short*>::Member) == 8, "three owner heads");
int main() { return 0; }
