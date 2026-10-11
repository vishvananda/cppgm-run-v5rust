namespace n { typedef unsigned long size_type; }
template<class T, T... I> struct sequence {};
template<n::size_type... I> using indices = sequence<n::size_type, I...>;
template<class R, n::size_type... I>
R f(R*, indices<I...>) { return R(); }
template<n::size_type... I>
void f(void*, indices<I...>) {}
int main() { f((void*)0, indices<0>()); }
