// VALIDATION: run-pass
// N3485 focus: 14.8.2.1 [temp.deduct.call], deduction through a
// namespace-qualified alias-template parameter on an inherited member template.

typedef unsigned long size_t;

namespace boost {
namespace mp11 {

template<class T, T V>
struct integral_constant
{
  static const T value = V;
};

template<size_t I>
using mp_size_t = integral_constant<size_t, I>;

}  // namespace mp11
}  // namespace boost

template<class... T>
class holder;

template<size_t I, class V>
struct alt;

template<size_t I, class... T>
struct alt<I, holder<T...> >
{
  typedef int type;
};

template<size_t I, class V>
using alt_t = typename alt<I, V>::type;

namespace detail {

template<bool B, class... T>
struct base_impl;

template<class... T>
using base = base_impl<false, T...>;

template<class... T>
struct base_impl<false, T...>
{
  int value;

  base_impl() : value(7) {}

  template<size_t I>
  alt_t<I, holder<T...> > &get(boost::mp11::mp_size_t<I>)
  {
    return value;
  }

  template<size_t I>
  alt_t<I, holder<T...> > const &get(boost::mp11::mp_size_t<I>) const
  {
    return value;
  }
};

template<class... T>
struct relay : public base<T...>
{
  using variant_base = base<T...>;
  using variant_base::variant_base;
};

}  // namespace detail

template<class... T>
class holder : private detail::relay<T...>
{
  using variant_base = detail::relay<T...>;

public:
  holder() : variant_base() {}

  using variant_base::get;
};

template<size_t I, class... T>
alt_t<I, holder<T...> > const &unsafe_get(holder<T...> const &v)
{
  return v.get(boost::mp11::mp_size_t<I>());
}

int main()
{
  holder<int> h;
  return unsafe_get<0>(h) == 7 ? 0 : 1;
}
