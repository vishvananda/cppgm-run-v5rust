#include <iterator>
#include <type_traits>
#include <vector>

struct traversal_tag {};
struct composite_input_category : std::input_iterator_tag, traversal_tag {};

struct component {
  component() {}
  component(int const&) {}
};

struct composite_input_iterator {
  typedef std::ptrdiff_t difference_type;
  typedef int value_type;
  typedef int const* pointer;
  typedef int const& reference;
  typedef composite_input_category iterator_category;

  int value;

  composite_input_iterator() : value(0) {}
  reference operator*() const { return value; }
  composite_input_iterator& operator++() { return *this; }
  composite_input_iterator operator++(int)
  {
    composite_input_iterator copy = *this;
    ++*this;
    return copy;
  }
};

bool operator==(composite_input_iterator, composite_input_iterator)
{
  return true;
}

bool operator!=(composite_input_iterator lhs, composite_input_iterator rhs)
{
  return !(lhs == rhs);
}

#ifdef _LIBCPP_VERSION
static_assert(
    std::__has_iterator_category_convertible_to<
        composite_input_iterator,
        std::input_iterator_tag>::value,
    "composite input category converts to input");
static_assert(
    !std::__has_iterator_category_convertible_to<
        composite_input_iterator,
        std::forward_iterator_tag>::value,
    "composite input category is not a forward iterator");
static_assert(
    std::__has_exactly_input_iterator_category<composite_input_iterator>::value,
    "composite category remains exactly input");
#endif

void vector_composite_input_anchor(composite_input_iterator first,
                                   composite_input_iterator last)
{
  std::vector<component> values(first, last);
  (void)values;
}

static_assert(sizeof(&vector_composite_input_anchor) > 0,
              "vector range constructor body anchor");
