#include <memory>

template<class T>
T&& declval();

template<class T>
auto probe(T*) -> decltype(declval<T>().size(), char());

long probe(...);

template<class T>
struct storage
{
  typedef std::allocator_traits<std::allocator<T> > traits;
  typedef typename traits::size_type size_type;
  typedef typename traits::template rebind_alloc<storage> node_allocator;
  static_assert(sizeof(node_allocator) != 0, "");
};

template<class T>
struct container : storage<T>
{
  typename storage<T>::size_type size() const;
};

static_assert(sizeof(probe((container<int>*)0)) == 1, "");
