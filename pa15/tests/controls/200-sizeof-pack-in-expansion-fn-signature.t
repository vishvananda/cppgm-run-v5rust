// N3485 5.3.3p5, 14.5.3p4-5: sizeof...(I) inside a pattern that expands I
// counts the whole pack; expect exit 0
template<int... I> struct S {};
template<int... I> S<(int(sizeof...(I)) - I)...> f(S<I...>);
template<int... I> int g(S<I...>) { return 0; }
int check(S<2, 1>) { return 1; }
int main() { return check(decltype(f(S<0, 1>()))()) == 1 ? 0 : 1; }
