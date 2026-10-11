// VALIDATION: compile-pass
// A member template imported with a class using-declaration must instantiate
// with the class where it was declared, not another specialization of the same
// class template that imported it.

struct IndexTag {};
struct VisitorTag {};
struct Graph {};
struct Map {};
struct Visitor {};

template<class Key, class Default>
struct default_ {
  const Default& value;
  explicit default_(const Default& v) : value(v) {}
};

struct empty_arg_list {
  template<class Key, class Default>
  const Default& operator[](const default_<Key, Default>& d) const
  {
    return d.value;
  }
};

template<class Key, class Arg>
struct tagged_argument {
  typedef Key key_type;
  typedef Arg value_type;
  typedef const Arg& reference;

  const Arg& value;
  explicit tagged_argument(const Arg& v) : value(v) {}
  reference get_value() const { return value; }
};

template<class TaggedArg, class Next = empty_arg_list>
struct arg_list : Next {
  typedef typename TaggedArg::key_type key_type;
  typedef typename TaggedArg::reference reference;

  TaggedArg arg;

  arg_list(const TaggedArg& head, const Next& tail) : Next(tail), arg(head) {}

  template<class Default>
  reference operator[](const default_<key_type, Default>&) const
  {
    return arg.get_value();
  }

  using Next::operator[];
};

template<class ArgType, bool Exists>
struct override_const_property_t {
  typedef ArgType result_type;
  result_type operator()(const Graph&, const ArgType& a) const { return a; }
};

template<class ArgPack>
typename override_const_property_t<Map, true>::result_type
override_const_property(const ArgPack& ap, const Graph& g)
{
  int fallback = 0;
  return override_const_property_t<Map, true>()(
      g, ap[default_<IndexTag, int>(fallback)]);
}

int main()
{
  Graph g;
  Map map;
  Visitor visitor;
  typedef tagged_argument<IndexTag, Map> TaggedIndex;
  typedef tagged_argument<VisitorTag, Visitor> TaggedVisitor;
  typedef arg_list<TaggedIndex> TailArgs;
  typedef arg_list<TaggedVisitor, TailArgs> Args;

  TaggedIndex tagged_index(map);
  TaggedVisitor tagged_visitor(visitor);
  TailArgs tail_args(tagged_index, empty_arg_list());
  Args args(tagged_visitor, tail_args);
  override_const_property(args, g);
  return 0;
}
