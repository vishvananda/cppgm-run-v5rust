namespace boost {
namespace xpressive {
namespace detail {

struct width {
  width(int = 0) {}
};

width operator|(width, width const &);

}

namespace regex_constants {

enum syntax_option_type {
  not_dot_newline = 1 << 12,
  not_dot_null = 1 << 11
};

enum match_flag_type {
  match_default = 0
};

syntax_option_type operator&(syntax_option_type, syntax_option_type);
match_flag_type operator&(match_flag_type, match_flag_type);

}

namespace detail {

template<typename T>
int check(regex_constants::syntax_option_type flags)
{
  using namespace regex_constants;
  return ((int)not_dot_newline | not_dot_null) & flags;
}

}
}
}

int main()
{
  return boost::xpressive::detail::check<int>(
             boost::xpressive::regex_constants::not_dot_null) ==
                 boost::xpressive::regex_constants::not_dot_null ? 0 : 1;
}
