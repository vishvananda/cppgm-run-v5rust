// PA16 focus: class partial-specialization matching deduces through a nested
// class-template-id argument instead of selecting the primary.

template<class First, class Second>
struct pair_like {
  First first;
  Second second;
};

namespace lib {

enum tree_type_enum { red_black_tree };

namespace detail {
struct empty_hook {};

template<class Node>
struct node_value {
  typedef typename Node::value_type type;
};

template<class VoidPointer, tree_type_enum TreeType, bool OptimizeSize>
struct hook;

template<class VoidPointer, bool OptimizeSize>
struct hook<VoidPointer, red_black_tree, OptimizeSize> {
  typedef empty_hook type;
};
}

template<unsigned long Size, unsigned long Align>
struct aligned_storage {
  char data[Size];
};

template<class T>
struct alignment_of {
  static const unsigned long value = 1;
};

template<class T, class HookDefiner, bool PairBased = false>
struct base_node : public HookDefiner::type {
  typedef T value_type;
  typedef typename aligned_storage<sizeof(T), alignment_of<T>::value>::type storage_t;
  storage_t storage;
};

namespace detail {
template<class T, class VoidPointer, tree_type_enum TreeType, bool OptimizeSize>
struct node_value<base_node<T, hook<VoidPointer, TreeType, OptimizeSize>, true> > {
  typedef T type;
};

template<class IIterator>
struct iterator_types {
  typedef typename IIterator::value_type it_value_type;
  typedef typename node_value<it_value_type>::type value_type;
};
}

}

namespace intrusive {
template<class Node>
struct traits {
  typedef Node value_type;
};

template<class Traits>
struct tree_iterator {
  typedef typename Traits::value_type value_type;
};
}

template<class Key, class Value>
struct map_like {
  typedef lib::base_node<
      pair_like<Key const, Value>,
      lib::detail::hook<void *, lib::red_black_tree, true>,
      true> node_t;
  typedef intrusive::tree_iterator<intrusive::traits<node_t> > iiterator;
  typedef typename lib::detail::iterator_types<iiterator>::value_type * iterator;
};

class recursive_map {
  map_like<recursive_map, recursive_map>::iterator it_;
};

int main()
{
  return 0;
}
