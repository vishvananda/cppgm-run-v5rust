template<bool B>
struct bool_constant
{
  static const bool value = B;
};

template<bool B>
struct traits
{
  using hash_cached = bool_constant<B>;
};

template<typename T>
struct node_alloc
{
  using value_type = T;
};

struct alloc
{
  template<typename U>
  struct rebind
  {
    using other = node_alloc<U>;
  };
};

template<typename...>
using void_t = void;

template<typename T, typename U, typename = void>
struct rebind
{
  using type = node_alloc<U>;
};

template<typename T, typename U>
struct rebind<T, U, void_t<typename T::template rebind<U>::other> >
{
  using type = typename T::template rebind<U>::other;
};

template<typename Alloc, typename T>
using alloc_rebind = typename rebind<Alloc, T>::type;

template<typename V, bool Cache>
struct hash_node {};

template<typename T, bool Use = !__is_final(T) && __is_empty(T)>
struct ebo_helper
{
  T value;
};

template<typename T>
struct ebo_helper<T, false>
{
  T value;
};

template<typename NodeAlloc>
struct table_alloc
{
  ebo_helper<NodeAlloc> alloc_value{};
  using node_type = typename NodeAlloc::value_type;
};

template<typename Alloc, typename V, typename Traits>
struct table
  : table_alloc<alloc_rebind<Alloc, hash_node<V, Traits::hash_cached::value> > >
{
  table() {}
};

int main()
{
  table<alloc, int, traits<true> > t;
  (void)t;
  return 0;
}
