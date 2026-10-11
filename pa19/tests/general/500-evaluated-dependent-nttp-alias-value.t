// N3485 focus: [temp.deduct], [temp.alias], and [temp.dep.expr].
// A constructor template can default a non-type parameter whose declared type
// comes from a dependent member alias. Once deduction evaluates the defaulted
// argument to a concrete bool, later host symbol naming should use that typed
// value instead of re-entering the dependent alias expression.
template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<bool Cond, class... Types>
struct constraints {
  template<class... Args>
  static constexpr bool constructible()
  {
    return true;
  }
};

template<class... Elements>
struct tuple {
  template<bool Cond>
  using tcc = constraints<Cond, Elements...>;

  template<bool Cond, class... Args>
  using implicit_ctor =
      enable_if_t<tcc<Cond>::template constructible<Args...>(), bool>;

  template<bool NotEmpty = (sizeof...(Elements) >= 1),
           implicit_ctor<NotEmpty, const Elements&...> = true>
  tuple(const Elements&... elements) {}
};

int main()
{
  int value = 1;
  tuple<int const &> t(value);
  return 0;
}
