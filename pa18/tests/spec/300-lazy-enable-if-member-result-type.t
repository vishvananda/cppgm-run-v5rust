// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], 14.8.2.8 [temp.deduct.type]
// A lazy enable_if result can name the nested ::type of a metafunction argument.

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

struct true_type {
  static const bool value = true;
};

template<class Cond, class MetaFn>
struct lazy_enable_if
    : enable_if<Cond::value, typename MetaFn::type> {};

template<class Tag, class T>
struct tagged_argument {
  typedef tagged_argument type;
  T value;

  explicit tagged_argument(T x) : value(x) {}
};

struct graph_tag {};

struct keyword {
  template<class T>
  typename lazy_enable_if<
      true_type,
      tagged_argument<graph_tag, T &> >::type
  operator=(T & value) const
  {
    return tagged_argument<graph_tag, T &>(value);
  }
};

int main()
{
  int value = 7;
  tagged_argument<graph_tag, int &> arg = keyword() = value;
  arg.value = 3;
  return value == 3 ? 0 : 1;
}
