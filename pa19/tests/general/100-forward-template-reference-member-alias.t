// Reduced from Boost.Proto via Boost.Xpressive.
// A class template specialization can be named while the primary template is
// only forward-declared, then later require member typedefs after the primary
// definition is available.

template<bool B>
struct bool_
{
  static const bool value = B;
};

typedef bool_<true> true_;
typedef bool_<false> false_;

template<class T>
struct not_value : bool_<!T::value> {};

struct any_tag;
struct address_of_tag;
struct shift_right_tag;
struct generator {};

template<class T>
struct list1
{
  typedef T child0;
};

template<class T, class U>
struct list2
{
  typedef T child0;
  typedef U child1;
};

template<class Tag, class Args, long N>
struct basic_expr
{
  typedef basic_expr proto_derived_expr;
  typedef basic_expr proto_grammar;
};

struct any
{
  typedef any proto_grammar;
};

template<class Grammar>
struct grammar_not
{
  typedef grammar_not proto_grammar;
};

template<class T>
struct address_of;

template<class Generator, class Grammar>
struct domain : Generator
{
  typedef Grammar proto_grammar;
};

struct regex_domain
  : domain<generator, grammar_not<address_of<any> > >
{};

template<class T>
struct address_of
{
  typedef basic_expr<address_of_tag, list1<T>, 1> proto_grammar;
};

namespace detail
{
  template<class Expr, class BasicExpr, class Grammar>
  struct matches_ : false_ {};

  template<class Expr, class BasicExpr>
  struct matches_<Expr, BasicExpr, any> : true_ {};

  template<class Expr, class BasicExpr, class Grammar>
  struct matches_<Expr, BasicExpr, grammar_not<Grammar> >
    : not_value<matches_<Expr, BasicExpr, typename Grammar::proto_grammar> >
  {};

  template<class Expr, class Tag, class Args1, class Args2>
  struct matches_<Expr, basic_expr<Tag, Args1, 1>, basic_expr<Tag, Args2, 1> >
    : matches_<
          typename Args1::child0::proto_derived_expr,
          typename Args1::child0::proto_grammar,
          typename Args2::child0::proto_grammar
      >
  {};
}

template<class Expr, class Grammar>
struct matches
  : detail::matches_<
        typename Expr::proto_derived_expr,
        typename Expr::proto_grammar,
        typename Grammar::proto_grammar
    >
{};

struct left
{
  typedef left proto_derived_expr;
  typedef basic_expr<any_tag, list1<any>, 0> proto_grammar;
};

struct right
{
  typedef right proto_derived_expr;
  typedef basic_expr<any_tag, list1<any>, 0> proto_grammar;
};

typedef basic_expr<shift_right_tag, list2<left const &, right const &>, 2> shift_expr;
typedef regex_domain::proto_grammar domain_grammar;

static_assert(matches<shift_expr, domain_grammar>::value, "domain grammar");

int main()
{
  return 0;
}
