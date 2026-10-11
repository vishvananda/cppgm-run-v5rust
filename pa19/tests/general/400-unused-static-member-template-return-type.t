struct random_access_tag {};

template <class Category>
struct category_map;

template <>
struct category_map<random_access_tag>
{
  typedef random_access_tag type;
};

template <class Derived, class Category>
struct iterator_facade
{
  typedef Category category;
};

template <class Iterator>
struct mpl_iterator
  : iterator_facade<
      mpl_iterator<Iterator>,
      typename category_map<typename Iterator::category>::type>
{
};

template <class Seq, int Pos>
struct map_iterator
  : iterator_facade<map_iterator<Seq, Pos>, typename Seq::category>
{
};

struct map
{
  struct category : random_access_tag {};
};

struct true_tag {};
struct false_tag {};

template <class T>
struct convert_iterator
{
  static T const& call(T const& x, true_tag)
  {
    return x;
  }

  static mpl_iterator<T> call(T const&, false_tag)
  {
    return mpl_iterator<T>();
  }

  static T const& call(T const& x)
  {
    return call(x, true_tag());
  }
};

int main()
{
  map_iterator<map const, 0> it;
  convert_iterator<map_iterator<map const, 0> >::call(it);
  return 0;
}
