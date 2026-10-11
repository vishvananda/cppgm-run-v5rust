// Regression: a reference-collected alias can name a class-template
// specialization whose member layout depends on the enclosing class layout.

template<class Key, class Value>
struct pair_like
{
  Key first;
  Value second;
};

template<class Key, class Value>
struct value_payload {};

template<class T>
struct get_node_value_type
{
  using type = T;
};

template<class Key, class Value>
struct get_node_value_type<value_payload<Key, Value> >
{
  using type = pair_like<const Key, Value>;
};

template<class T>
using get_node_value_type_t = typename get_node_value_type<T>::type;

template<class T>
struct tree_base
{
  using value_type = get_node_value_type_t<T>;
};

template<class Key, class Value>
struct map_like
{
  typedef tree_base<value_payload<Key, Value> > base_type;
  typedef typename base_type::value_type value_type;
  base_type impl;
};

template<class CharT>
struct parse_tree
{
  typedef map_like<CharT, parse_tree> children_type;
  typedef typename children_type::value_type value_type;

  children_type children;
  short value;
};

typedef parse_tree<char>::value_type selected_value_type;

int main()
{
  parse_tree<char> t;
  return sizeof(t) == 0;
}
