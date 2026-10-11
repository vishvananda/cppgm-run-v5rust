// VALIDATION: compile-pass
template<class, class> struct base;
template<class R, class A> struct base<R, R(A...)> { typedef int type; };
template<class> struct outer;
template<class R, class A> struct outer<R(A...)> : base<R, R(A...)> {};
typedef outer<int(char...)>::type result;
int main() {}
