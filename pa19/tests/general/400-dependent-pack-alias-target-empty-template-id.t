// VALIDATION: compile-pass
// N3485 focus: 14.5.7 [temp.alias]

template<class... T>
struct tuple
{
};

template<class T>
using id_t = T;

template<class... T>
struct impl
{
};

template<class... Ts>
using cat_t = typename impl<tuple<>, id_t<Ts>...>::type;

template<class T0, class... Ts>
cat_t<T0, Ts...> f(T0 && t0, Ts &&... ts)
{
  using R = cat_t<T0, Ts...>;
  return R();
}

int main()
{
  return 0;
}
