// VALIDATION: compile-pass
// Completing a referenced nested class template specialization must collect
// out-of-class member typedefs before resolving later qualified member types.

template<class Derived, class Base, class Value>
struct adaptor
{
  typedef Base base_type;
};

struct simple_container
{
  struct iterator {};
};

template<class Key, class Data, class Compare>
struct ptree
{
  typedef int value_type;
  struct subs;
  class iterator;
};

template<class K, class D, class C>
struct ptree<K, D, C>::subs
{
  typedef simple_container base_container;
};

template<class K, class D, class C>
class ptree<K, D, C>::iterator
  : public adaptor<iterator, typename subs::base_container::iterator, value_type>
{
  typedef adaptor<iterator, typename subs::base_container::iterator, value_type> baset;

public:
  typedef typename baset::base_type base_type;
};

int main()
{
  (void)sizeof(ptree<int, int, int>::iterator);
  return 0;
}
