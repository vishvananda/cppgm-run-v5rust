namespace imported {
namespace aux {}
}

namespace meta {
namespace mpl {
using namespace imported;
namespace aux {
using namespace imported::aux;
}
}
}

namespace meta {
namespace mpl {
namespace aux {

template<bool C, typename T1, typename T2>
struct if_c {
  typedef T1 type;
};

template<typename T1, typename T2>
struct if_c<false, T1, T2> {
  typedef T2 type;
};

template<typename T>
struct rank;

template<>
struct rank<long> {
  static const int value = 1;
};

template<typename T1, typename T2>
struct largest_int
  : if_c<(rank<T1>::value >= rank<T2>::value), T1, T2> {};

}
}
}

namespace meta {
namespace mpl {

struct tag {};

template<typename T, T Value>
struct integral_c {
  static const T value = Value;
};

template<typename Tag1, typename Tag2>
struct impl;

template<>
struct impl<tag, tag> {
  template<typename N1, typename N2>
  struct apply
    : integral_c<typename aux::largest_int<long, long>::type,
                 (N1::value ^ N2::value)> {};
};

}
}

namespace local {
template<long Value>
struct constant {
  static const long value = Value;
};
}

typedef meta::mpl::impl<meta::mpl::tag, meta::mpl::tag>
    ::apply<local::constant<7>, local::constant<3> > result;

static_assert(result::value == 4, "");

int main()
{
  return result::value == 4 ? 0 : 1;
}
