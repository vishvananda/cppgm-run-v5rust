// N3485 14.5.5.3/14.5.2: each enclosing template-head is declared before
// probing its partial-specialization qualifier; expect exit 0
template<class A> struct Outer { template<class B> struct Inner; };
template<class A> template<class B> struct Outer<A>::Inner<B*> { struct Member; };
template<class P> template<class Q> struct Outer<P>::Inner<Q*>::Member {
  P first;
  Q second;
};
static_assert(sizeof(Outer<int>::Inner<char*>::Member) == 8, "partial owner arguments");
static_assert(sizeof(Outer<char>::Inner<long*>::Member) == 16, "independent owner arguments");
int main() { return 0; }
