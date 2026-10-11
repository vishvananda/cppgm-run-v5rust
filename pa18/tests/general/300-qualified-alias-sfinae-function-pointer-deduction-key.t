struct false_type {
  static const bool value = false;
};

struct true_type {
  static const bool value = true;
};

template<class...>
struct make_void {
  typedef void type;
};

template<class... T>
using void_t = typename make_void<T...>::type;

namespace traits {
namespace detail {

template<class T>
using describe = typename T::missing;

template<class T, class Enable = void>
struct has_description : false_type {
};

template<class T>
struct has_description<T, void_t<describe<T> > > : true_type {
};

}

template<class T>
using has_description = detail::has_description<T>;

}

namespace lib {

template<class T, class U>
struct pair {
};

}

namespace traits {
namespace detail {

template<class T>
struct remove_const {
  typedef T type;
};

template<class T>
struct remove_const<T const> {
  typedef T type;
};

template<class T>
struct is_pair_like : false_type {
};

template<class T, class U>
struct is_pair_like<lib::pair<T, U> > : true_type {
};

}

template<class T>
struct is_pair_like : detail::is_pair_like<typename detail::remove_const<T>::type> {
};

}

enum E1 {
  e1
};

enum class E2 {
  e2
};

namespace boost {
namespace detail {

template<class T>
int test_trait_impl(const char *, void (*)(T), bool expected)
{
  return T::value == expected ? 0 : 1;
}

}
}

#define TEST_TRAIT_FALSE(type) (::boost::detail::test_trait_impl(#type, (void(*)type)0, false))
#define TEST_TRAIT_TRUE(type) (::boost::detail::test_trait_impl(#type, (void(*)type)0, true))

int main()
{
  using traits::has_description;
  using traits::is_pair_like;
  return TEST_TRAIT_FALSE((has_description<E1>)) +
         TEST_TRAIT_FALSE((has_description<E1 const>)) +
         TEST_TRAIT_FALSE((has_description<E2>)) +
         TEST_TRAIT_FALSE((has_description<E2 const>)) +
         TEST_TRAIT_TRUE((is_pair_like<lib::pair<int, E1> >)) +
         TEST_TRAIT_TRUE((is_pair_like<lib::pair<int, E1> const>));
}
