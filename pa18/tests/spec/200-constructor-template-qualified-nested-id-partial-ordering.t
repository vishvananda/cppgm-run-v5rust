// VALIDATION: compile-pass
// N3485 focus: 14.8.2.4 [temp.deduct.partial]
// A constructor template with a nested concrete template-id parameter is more
// specialized than a generic wrapper constructor when both instantiate to the
// same concrete parameter type.

namespace vendor {
namespace detail {

template<class T>
struct wrap_iter
{
};

}
}

namespace lib {

template<class T>
struct counted_ptr
{
  T * value;
};

template<class Char, class Impl>
struct traits
{
};

template<class Char>
struct traits_impl
{
};

namespace detail {

template<class Matcher, class Iter>
struct dynamic_node
{
};

template<class Iter>
struct alternates
{
};

template<class Range, class Traits>
struct alternate_matcher
{
};

template<class Iter>
struct sequence
{
  int selected;

  template<class Matcher>
  sequence(counted_ptr<dynamic_node<Matcher, Iter> > const &)
    : selected(1)
  {
  }

  template<class Traits>
  sequence(counted_ptr<dynamic_node<alternate_matcher<alternates<Iter>, Traits>, Iter> > const &)
    : selected(2)
  {
  }
};

template<class Iter, class Matcher>
sequence<Iter> make_dynamic(Matcher const &)
{
  typedef dynamic_node<Matcher, Iter> node_type;
  counted_ptr<node_type> ptr = {};
  return sequence<Iter>(ptr);
}

}
}

int main()
{
  typedef vendor::detail::wrap_iter<char const *> iter;
  typedef lib::traits<char, lib::traits_impl<char> > traits_type;
  lib::detail::alternate_matcher<lib::detail::alternates<iter>, traits_type> matcher = {};
  lib::detail::sequence<iter> seq = lib::detail::make_dynamic<iter>(matcher);
  return seq.selected == 2 ? 0 : 1;
}
