// VALIDATION: compile-pass
// N3485 focus: 14.5.3 [temp.variadic], 5.1.1 [expr.prim]

template<class T, T v>
struct integral_constant {
  static constexpr T value = v;
};

template<class T, T v>
constexpr T integral_constant<T, v>::value;

template<class... T>
struct list {
};

struct options {
  bool a;
  bool b;
};

options make_options(bool a, bool b)
{
  options out = { a, b };
  return out;
}

template<class... Bools>
options make_options(list<Bools...>)
{
  return make_options(Bools::value...);
}

int main()
{
  options out =
      make_options(list<
          integral_constant<bool, true>,
          integral_constant<bool, false> >());
  return out.a && !out.b ? 0 : 1;
}
