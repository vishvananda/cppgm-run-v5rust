// VALIDATION: compile-pass
// N3485 focus: 14.8.2.4 [temp.deduct.partial], 14.8.3 [temp.over]
// Boost.Parameter-shaped member operator partial ordering must prefer a
// defaulted keyword overload with a fixed tag over the generic fallback.

struct weight_tag {};
struct predecessor_tag {};

struct supplied_map
{
  char data[1];
};

struct default_map
{
  char data[2];
};

template<class K, class Default>
struct default_
{
  Default & value;

  default_(Default & v) : value(v) {}
};

template<class K>
struct keyword
{
  template<class Default>
  default_<K, Default> operator|(Default & d) const
  {
    return default_<K, Default>(d);
  }
};

template<class Keyword, class T>
struct tagged_argument
{
  typedef Keyword key_type;
  typedef T const & reference;

  T const & value;

  tagged_argument(T const & v) : value(v) {}

  reference get_value() const
  {
    return value;
  }

  reference operator[](keyword<Keyword> const &) const
  {
    return get_value();
  }

  template<class Default>
  reference operator[](default_<key_type, Default> const &) const
  {
    return get_value();
  }

  template<class KW, class Default>
  Default & operator[](default_<KW, Default> const & x) const
  {
    return x.value;
  }
};

struct empty_arg_list
{
  template<class K, class Default>
  Default & operator[](default_<K, Default> x) const
  {
    return x.value;
  }
};

template<class Tagged, class Next>
struct arg_list : Next
{
  typedef typename Tagged::key_type key_type;
  typedef typename Tagged::reference reference;

  Tagged arg;

  using Next::operator[];

  arg_list(Tagged const & tagged, Next const & next) : Next(next), arg(tagged) {}

  reference operator[](keyword<key_type> const &) const
  {
    return arg.get_value();
  }

  template<class Default>
  reference operator[](default_<key_type, Default> const &) const
  {
    return arg.get_value();
  }
};

keyword<weight_tag> _weight;
keyword<predecessor_tag> _predecessor;

int main()
{
  supplied_map supplied;
  default_map fallback;
  int predecessor_value = 0;

  typedef arg_list<
      tagged_argument<weight_tag, supplied_map>,
      arg_list<tagged_argument<predecessor_tag, int>, empty_arg_list> >
      pack_type;

  pack_type pack = pack_type(
      tagged_argument<weight_tag, supplied_map>(supplied),
      arg_list<tagged_argument<predecessor_tag, int>, empty_arg_list>(
          tagged_argument<predecessor_tag, int>(predecessor_value),
          empty_arg_list()));

  supplied_map const & selected = pack[_weight | fallback];
  return sizeof(selected) == sizeof(supplied_map) ? 0 : 1;
}
