// VALIDATION: compile-pass
template<class> struct inner;
template<class R, class... A> struct inner<R(A...)> { typedef int type; };
template<class> struct outer;
template<class R, class A> struct outer<R(A)> { typedef A type; };
template<class... A> typename outer<void(typename inner<void(A...)>::type)>::type call(A...) { return 0; }
static_assert(sizeof(call(1, 2)) == sizeof(int), "");
int main() {}
