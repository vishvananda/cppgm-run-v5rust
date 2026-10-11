// N3485 focus: 14.8.2 substitution failure and alias-template substitution.
// A cv-qualified alias-template target that becomes a reference must preserve
// the cv qualification of the referenced type while checking defaulted SFINAE
// parameters.
template <bool B, class T = void>
struct enable_if {};

template <class T>
struct enable_if<true, T> {
  typedef T type;
};

template <bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template <class A, class B>
struct is_same {
  static const bool value = false;
};

template <class A>
struct is_same<A, A> {
  static const bool value = true;
};

template <class T>
struct remove_reference {
  typedef T type;
};

template <class T>
struct remove_reference<T &> {
  typedef T type;
};

template <class T>
using remove_reference_t = typename remove_reference<T>::type;

template <class T>
using make_const_lvalue_ref = const remove_reference_t<T> &;

struct item {};

template <class T,
          enable_if_t<is_same<const item &, make_const_lvalue_ref<T> >::value,
                      int> = 0>
struct selected {
  static const bool value = true;
};

static_assert(selected<item &>::value, "");

int main()
{
  return selected<item &>::value ? 0 : 1;
}
