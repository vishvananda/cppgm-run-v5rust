template<class T>
struct has_size_type {
  typedef char no_type;
  struct yes_type {
    char dummy[2];
  };

  template<class C>
  static yes_type test(typename C::size_type x);

  template<class C>
  static no_type test(...);

  static const bool value = sizeof(test<T>(0)) == sizeof(yes_type);
};

struct with_size_type {
  typedef int size_type;
};

static_assert(!has_size_type<char const[5]>::value,
              "array type has no size_type member");
static_assert(has_size_type<with_size_type>::value,
              "class member type remains viable");

int main() {
  return has_size_type<char const[5]>::value ? 1 : 0;
}
