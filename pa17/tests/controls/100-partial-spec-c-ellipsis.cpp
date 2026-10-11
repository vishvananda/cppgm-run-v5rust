// N3485 8.3.5p5/14.8.2.5: a function type's C ellipsis is part of it, so
// A<int(*)(int,int)> matches only R(*)(As...), not R(*)(As......) (libstdc++
// std::bind's _Bind_check_arity); expect exit 0
template<class F> struct A { static const int value = 0; };
template<class R, class... As> struct A<R (*)(As...)> { static const int value = 1; };
template<class R, class... As> struct A<R (*)(As......)> { static const int value = 2; };
int f(int, int);
int g(int, ...);
int main() { return (A<decltype(&f)>::value == 1 && A<decltype(&g)>::value == 2) ? 0 : 1; }
