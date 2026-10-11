template<class T>
struct Alloc {
  typedef T value_type;
};

template<class Allocator>
struct AllocTraits {
  typedef typename Allocator::value_type * pointer;
};

template<class Value, class ValuePtr>
struct NodeTraits {
  typedef ValuePtr BasePtr;
};

template<class First, class Second>
struct Pair {
  First first;
  Second second;
};

template<class T>
struct Identity {
  const T & operator()(const T & value) const { return value; }
};

template<class Key, class Value, class KeyOfValue, class Allocator>
struct Tree {
  typedef typename AllocTraits<Allocator>::pointer ValuePtr;
  typedef NodeTraits<Value, ValuePtr> NodeTraitsAlias;
  typedef typename NodeTraitsAlias::BasePtr BasePtr;

  struct Iterator {
    BasePtr node;
  };

  template<class Arg, class NodeGen>
  Iterator insert_unique(Arg & value, NodeGen & gen);

  Pair<BasePtr, BasePtr> get_insert_pos(const Key &)
  {
    Pair<BasePtr, BasePtr> result = { 0, 0 };
    return result;
  }

  Iterator make_iterator(BasePtr ptr)
  {
    Iterator it = { ptr };
    return it;
  }
};

template<class Key, class Value, class KeyOfValue, class Allocator>
template<class Arg, class NodeGen>
typename Tree<Key, Value, KeyOfValue, Allocator>::Iterator
Tree<Key, Value, KeyOfValue, Allocator>::insert_unique(Arg & value, NodeGen & gen)
{
  (void)gen;
  Pair<BasePtr, BasePtr> result = get_insert_pos(KeyOfValue()(value));
  return make_iterator(result.first);
}

struct Value {
  int key;
};

struct Generator {};

int main()
{
  Tree<Value, Value, Identity<Value>, Alloc<Value> > tree;
  Value value = { 7 };
  Generator gen;
  Tree<Value, Value, Identity<Value>, Alloc<Value> >::Iterator it =
      tree.insert_unique(value, gen);
  (void)it;
}
