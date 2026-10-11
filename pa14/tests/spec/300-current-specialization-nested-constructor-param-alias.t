// VALIDATION: compile-pass
// A concrete nested class instantiated from an out-of-class definition can
// still name its enclosing current specialization through source-local
// template parameter aliases.

struct simple_container
{
  struct iterator {};
  struct const_iterator {};
};

template<class Key, class Data, class Compare>
struct basic_tree
{
  struct subs;
  class iterator;
  class const_iterator;
  class reverse_iterator;
  class const_reverse_iterator;
};

template<class K, class D, class C>
struct basic_tree<K, D, C>::subs
{
  typedef simple_container base_container;
};

template<class K, class D, class C>
class basic_tree<K, D, C>::iterator
{
public:
  typedef typename subs::base_container::iterator base_type;
  iterator() {}
};

template<class K, class D, class C>
class basic_tree<K, D, C>::const_iterator
{
public:
  typedef typename subs::base_container::const_iterator base_type;
  const_iterator() {}
  const_iterator(iterator) {}
};

template<class K, class D, class C>
class basic_tree<K, D, C>::reverse_iterator
{
public:
  reverse_iterator() {}
  explicit reverse_iterator(iterator) {}
};

template<class K, class D, class C>
class basic_tree<K, D, C>::const_reverse_iterator
{
public:
  const_reverse_iterator() {}
  explicit const_reverse_iterator(const_iterator) {}
  const_reverse_iterator(typename basic_tree<K, D, C>::reverse_iterator) {}
};

int main()
{
  basic_tree<int, int, int>::reverse_iterator r;
  basic_tree<int, int, int>::const_reverse_iterator cr(r);
  return 0;
}
