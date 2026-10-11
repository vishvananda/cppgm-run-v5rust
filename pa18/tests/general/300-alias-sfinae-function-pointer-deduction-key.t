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

enum E1 {
  e1
};

enum class E2 {
  e2
};

template<class T>
int test_trait_impl(const char *, void (*)(T), bool expected)
{
  return T::value == expected ? 0 : 1;
}

#define TEST_TRAIT_FALSE(type) test_trait_impl(#type, (void(*)type)0, false)

int main()
{
  using traits::has_description;
  return TEST_TRAIT_FALSE((has_description<E1>)) +
         TEST_TRAIT_FALSE((has_description<E2>));
}
