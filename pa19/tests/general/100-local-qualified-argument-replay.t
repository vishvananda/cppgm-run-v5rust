// Reduced from Boost.Proto via Boost.Xpressive.
// Resolved qualified type annotations for template arguments must not be reused
// when a nested argument names an instantiation-local type such as impl::expr.

namespace proto {
template <typename A, typename B>
struct list2 {};

template <typename Tag, typename Args, int Arity>
struct expr;

template <typename Tag, typename A, typename B>
struct expr<Tag, list2<A, B>, 2> {
  typedef B proto_child1;
  A child0;
  B child1;
};

template <typename T>
struct transform_impl {
  typedef T expr;
  typedef T expr_param;
};

namespace result_of {
template <typename Expr, int N>
struct child_c;

template <typename Expr>
struct child_c<Expr const &, 1> {
  typedef typename Expr::proto_child1 type;
};

template <typename Expr>
struct right : child_c<Expr, 1> {};
}

template <typename Expr>
typename result_of::right<Expr &>::type right(Expr &e) {
  return e.child1;
}

struct tag_shift_right {};

template <typename A, typename B>
struct shift_right {
  typedef expr<tag_shift_right, list2<A, B>, 2> type;
};
}

struct begin_expr { explicit begin_expr(int) {} };
struct end_expr { explicit end_expr(int) {} };
struct tag_assign {};
struct mark_type {};
struct plus_leaf {};
struct deref_leaf {};

typedef proto::expr<tag_assign, proto::list2<mark_type const &, plus_leaf &>, 2>
    assign_plus;
typedef proto::expr<tag_assign, proto::list2<mark_type const &, deref_leaf &>, 2>
    assign_deref;

struct as_marker {
  template <typename Expr>
  struct impl : proto::transform_impl<Expr> {
    typedef
        typename proto::shift_right<
            begin_expr,
            typename proto::shift_right<
                typename proto::result_of::right<typename impl::expr>::type,
                end_expr
            >::type
        >::type
        result_type;

    result_type operator()(typename impl::expr_param expr) const {
      begin_expr begin(1);
      end_expr end(1);
      result_type that = {begin, {proto::right(expr), end}};
      return that;
    }
  };
};

int main() {
  mark_type mark = {};
  plus_leaf plus = {};
  deref_leaf deref = {};
  assign_plus ap = {mark, plus};
  assign_deref ad = {mark, deref};
  as_marker::impl<assign_plus const &>()(ap);
  as_marker::impl<assign_deref const &>()(ad);
  return 0;
}

// The same typed-owner path must preserve nested member template arguments
// when building object symbols.

template<typename T>
struct replay_outer
{
    template<typename Ref = T &>
    struct inner
    {
        void member();
    };
};

struct replay_access
{
    template<typename Facade>
    static void increment(Facade &f)
    {
        f.member();
    }
};

void use_default_replay(replay_outer<int>::inner<> &it)
{
    replay_access::increment(it);
}

void use_const_replay(replay_outer<int>::inner<int const &> &it)
{
    replay_access::increment(it);
}
