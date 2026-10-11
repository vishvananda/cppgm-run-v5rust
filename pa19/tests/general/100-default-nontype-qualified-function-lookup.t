// Reduced from Boost.Proto via Boost.Xpressive.
// A default non-type template argument can evaluate a qualified function call
// in sizeof while substituting previously bound template parameters.

namespace boost { namespace proto { namespace detail {

template<int N>
struct sized_type {
  typedef char (&type)[N];
};

struct default_domain {};
struct basic_default_domain {};

template<typename Domain>
struct domain_ {};

template<>
struct domain_<default_domain> {};

template<>
struct domain_<basic_default_domain> {};

sized_type<1>::type default_test(void *, void *);
sized_type<4>::type default_test(domain_<default_domain> *, domain_<default_domain> *);

template<typename D0, typename D1,
         int DefaultCase = sizeof(proto::detail::default_test((domain_<D0> *)0,
                                                              (domain_<D1> *)0))>
struct common_domain2 {
  static int const value = DefaultCase;
};

}}}

static_assert(boost::proto::detail::common_domain2<
                  boost::proto::detail::default_domain,
                  boost::proto::detail::default_domain>::value == 4,
              "qualified default non-type argument lookup");

int main()
{
  return 0;
}
