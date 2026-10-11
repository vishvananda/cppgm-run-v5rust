namespace std {
template<class T> struct is_scalar { static const bool value = false; };
template<> struct is_scalar<int> { static const bool value = true; };

template<class A, class B> struct is_same { static const bool value = false; };
template<class A> struct is_same<A, A> { static const bool value = true; };
}

namespace boost {
namespace mp11 {
struct mp_false { static const bool value = false; };
struct mp_true { static const bool value = true; };

namespace detail {
template<bool C, class T, class... E> struct mp_if_c_impl {};
template<class T, class... E> struct mp_if_c_impl<true, T, E...> { typedef T type; };
template<class T, class E> struct mp_if_c_impl<false, T, E> { typedef E type; };
}

template<class C, class T, class... E>
using mp_if = typename detail::mp_if_c_impl<static_cast<bool>(C::value), T, E...>::type;
}

template<bool B, class T = void> struct enable_if_c { typedef T type; };
template<class T> struct enable_if_c<false, T> {};
template<class Cond, class T = void> struct enable_if : enable_if_c<Cond::value, T> {};

namespace parameter {
struct in_reference;
struct consume_reference;
struct forward_reference;

namespace aux {
template<class Keyword, class Value>
struct default_ {
  default_(Value& x) : value(x) {}
  Value& value;
};

template<class Keyword, class Value>
struct default_r_ {
  default_r_(Value&& x) : value(static_cast<Value&&>(x)) {}
  Value&& value;
};
}

template<class Tag>
struct keyword {
  template<class Default>
  typename boost::enable_if<
      boost::mp11::mp_if<
          std::is_scalar<Default>,
          boost::mp11::mp_true,
          boost::mp11::mp_if<
              std::is_same<typename Tag::qualifier, in_reference>,
              boost::mp11::mp_true,
              std::is_same<typename Tag::qualifier, forward_reference> > >,
      aux::default_<Tag, Default const> >::type
  operator|(Default const& d) const
  {
    return aux::default_<Tag, Default const>(d);
  }

  template<class Default>
  typename boost::enable_if<
      boost::mp11::mp_if<
          std::is_scalar<Default>,
          boost::mp11::mp_false,
          boost::mp11::mp_if<
              std::is_same<typename Tag::qualifier, consume_reference>,
              boost::mp11::mp_true,
              std::is_same<typename Tag::qualifier, forward_reference> > >,
      aux::default_r_<Tag, Default> >::type
  operator|(Default&& d) const
  {
    return aux::default_r_<Tag, Default>(static_cast<Default&&>(d));
  }

  static keyword<Tag> const instance;
};

template<class Tag>
keyword<Tag> const keyword<Tag>::instance = keyword<Tag>();
}
}

namespace boost { namespace graph { namespace keywords { namespace tag {
struct color_map {
  typedef boost::parameter::in_reference qualifier;
};
}}}}

int main()
{
  boost::parameter::keyword<boost::graph::keywords::tag::color_map> kw;
  kw | 0;
  return 0;
}
