// Reduced from Boost.Proto via Boost.Xpressive.
// A partial specialization may omit a primary template argument whose default is
// a function type. Collecting the partial specialization must preserve that
// default argument as source text instead of instantiating its return type.

template<class Expr>
struct tag_of {
  typedef typename Expr::proto_tag type;
};

struct wildcard {};

template<class T>
struct transform {};

template<class Cases, class Transform = tag_of<wildcard>()>
struct switch_;

template<class Cases, class Transform>
struct switch_ : transform<switch_<Cases, Transform> > {};

template<class Cases>
struct switch_<Cases> : transform<switch_<Cases> > {};

int main()
{
  return 0;
}
