// N3485 5.3.3p5, 14.5.3p5: selected signature operands keep whole-pack queries;
// expect exit 0
template<int... I> struct Sequence {};
template<int... I> Sequence<(int(sizeof...(I)) - I)...> transform(Sequence<I...>);
int check(Sequence<2, 1>) { return 1; }
int check(Sequence<1, 2>) { return 2; }
int main()
{
  return check(decltype(transform(Sequence<0, 1>()))()) != 1 ||
         check(decltype(transform(Sequence<1, 0>()))()) != 2 ||
         check(decltype(transform(Sequence<0, 1>()))()) != 1;
}
