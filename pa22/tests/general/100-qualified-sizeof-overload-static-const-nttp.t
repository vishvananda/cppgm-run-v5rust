// Reduced from Boost.Range pure_iterator_traversal. In an open namespace,
// sizeof(ns::overload((T()))) must parse as a call expression, not as a
// function type-id.

typedef unsigned long size_t;

namespace std {
struct input_iterator_tag {};
}

namespace boost {
namespace iterators {
struct incrementable_traversal_tag {};
struct bidirectional_traversal_tag : incrementable_traversal_tag {};
struct random_access_traversal_tag : bidirectional_traversal_tag {};

namespace detail {
template<class Category, class Traversal>
struct iterator_category_with_traversal : Category, Traversal {};
}
}

typedef iterators::incrementable_traversal_tag incrementable_traversal_tag;
typedef iterators::bidirectional_traversal_tag bidirectional_traversal_tag;
typedef iterators::random_access_traversal_tag random_access_traversal_tag;

template<class IteratorT>
struct iterator_traversal;

template<class T>
struct iterator_traversal<T *>
{
  typedef random_access_traversal_tag type;
};

struct composite_iterator {};

template<>
struct iterator_traversal<composite_iterator>
{
  typedef iterators::detail::iterator_category_with_traversal<
      std::input_iterator_tag,
      iterators::bidirectional_traversal_tag> type;
};

namespace iterator_range_detail {

typedef char (&incrementable_t)[1];
typedef char (&bidirectional_t)[2];
typedef char (&random_access_t)[3];

incrementable_t test_traversal_tag(boost::incrementable_traversal_tag);
bidirectional_t test_traversal_tag(boost::bidirectional_traversal_tag);
random_access_t test_traversal_tag(boost::random_access_traversal_tag);

template<size_t S>
struct pure_iterator_traversal_impl
{
  typedef boost::incrementable_traversal_tag type;
};

template<>
struct pure_iterator_traversal_impl<sizeof(bidirectional_t)>
{
  typedef boost::bidirectional_traversal_tag type;
};

template<>
struct pure_iterator_traversal_impl<sizeof(random_access_t)>
{
  typedef boost::random_access_traversal_tag type;
};

template<class IteratorT>
struct pure_iterator_traversal
{
  typedef typename iterator_traversal<IteratorT>::type traversal_t;
  static const size_t traversal_i =
      sizeof(iterator_range_detail::test_traversal_tag((traversal_t())));
  typedef typename pure_iterator_traversal_impl<traversal_i>::type type;
};

}
}

int main()
{
  boost::iterator_range_detail::pure_iterator_traversal<int *>::type value;
  (void)value;
  boost::iterator_range_detail::pure_iterator_traversal<
      boost::composite_iterator>::type composite_value;
  boost::bidirectional_traversal_tag & composite_ref = composite_value;
  (void)composite_ref;
  return 0;
}
