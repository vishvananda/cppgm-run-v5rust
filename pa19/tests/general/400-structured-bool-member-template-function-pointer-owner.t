// Reduced from Boost.Asio/std::tuple: a structured bool expression may name a
// member class template whose owner has a function-pointer type argument.  The
// owner type must be resolved from carried semantic type syntax, not reparsed
// from text.

template<bool B>
struct bool_constant {
  static const bool value = B;
};

typedef bool_constant<false> false_type;

template<class T, class U>
struct is_same : false_type {};

template<class T>
struct is_same<T, T> : bool_constant<true> {};

template<class T>
struct remove_ref {
  typedef T type;
};

template<class T>
struct remove_ref<T&> {
  typedef T type;
};

template<class Pred>
struct not_ : bool_constant<!Pred::value> {};

template<class... T>
struct tuple {
  template<class... U>
  struct is_this_tuple : false_type {};

  template<class U>
  struct is_this_tuple<U> : is_same<typename remove_ref<U>::type, tuple> {};
};

struct error_code {};
struct endpoint {};

typedef bool (*condition_ptr)(const error_code&, const endpoint&);
typedef bool (&condition_ref)(const error_code&, const endpoint&);

static_assert(
    not_<tuple<int, condition_ptr>::is_this_tuple<int, condition_ref> >::value,
    "function pointer owner argument");

int main()
{
  return 0;
}
