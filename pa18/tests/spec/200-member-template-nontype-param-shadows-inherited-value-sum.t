// VALIDATION: compile-pass
// N3485 focus: 14.5.2 [temp.mem], 14.8.2 [temp.deduct]
// During instantiated member-template signature refresh, the member-template
// non-type parameter must still shadow inherited template-parameter values.

namespace mpl_
{
template<unsigned long N>
struct size_t
{
};
}

namespace detail
{
typedef mpl_::size_t<1073741822> unknown_width;

struct result
{
  unsigned long value;
  result(unsigned long v = 0): value(v) {}
};

template<unsigned long Width>
struct base_value
{
  static const unsigned long width = Width;
};

struct matcher : base_value<0>
{
};

struct next : base_value<3>
{
};

template<class Matcher, class Next>
struct expression : Matcher
{
  static const unsigned long width =
    Matcher::width != 1073741822 && Next::width != 1073741822 ?
      Matcher::width + Next::width : 1073741822;

  result get_width() const
  {
    return this->get_width_(mpl_::size_t<width>());
  }

  template<unsigned long Width>
  result get_width_(mpl_::size_t<Width>) const
  {
    return result(Width);
  }

  result get_width_(unknown_width) const
  {
    return result(100);
  }
};
}

int main()
{
  detail::expression<detail::matcher, detail::next> expr;
  return expr.get_width().value == 3 ? 0 : 1;
}
