// VALIDATION: compile-pass
// N3485 focus: 14.5.1.3 [temp.static]

namespace n
{
  template<class Tag>
  struct keyword
  {
    keyword() {}
    static ::n::keyword<Tag> const instance;
  };

  template<class Tag>
  ::n::keyword<Tag> const ::n::keyword<Tag>::instance = ::n::keyword<Tag>();
}

namespace tag
{
  struct color_map {};
}

namespace
{
  ::n::keyword<tag::color_map> const& color_map =
      ::n::keyword<tag::color_map>::instance;
}

int main()
{
  return &color_map == &::n::keyword<tag::color_map>::instance ? 0 : 1;
}
