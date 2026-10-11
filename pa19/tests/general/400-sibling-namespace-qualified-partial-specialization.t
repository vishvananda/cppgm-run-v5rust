// Reduced from Boost.Fusion vector. A partial specialization declared in a
// nested implementation namespace can name a sibling namespace in its
// class-template-id pattern.
namespace outer {
namespace detail {
template<unsigned long... I>
struct seq {};
}

namespace impl {
template<class Sequence, class... T>
struct data {
  static const int value = 0;
};

template<unsigned long... I, class... T>
struct data<detail::seq<I...>, T...> {
  static const int value = sizeof...(I) + sizeof...(T);
};
}

template<class... T>
struct vector : impl::data<detail::seq<0, 1, 2>, T...> {};
}

int main()
{
  return outer::vector<int, short, double>::value == 6 ? 0 : 1;
}
