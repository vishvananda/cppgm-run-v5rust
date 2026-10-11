// VALIDATION: compile-pass
// A member typedef may hide an outer class-template name while a nested
// specialization of the enclosing template is being collected.

template<class Key, class Value>
struct table
{
  typedef Key key_type;
  typedef Value mapped_type;

  Key key;
  Value value;
};

template<class Key, class Value>
struct make_table
{
  typedef table<Key, Value> type;
};

template<class Key, class Value>
struct map
{
  typedef typename make_table<Key, Value>::type table;

  table storage;
  typedef typename table::key_type key_type;
  typedef typename table::mapped_type mapped_type;
};

map<unsigned long, map<int, unsigned long> > values;

int main()
{
  return 0;
}
