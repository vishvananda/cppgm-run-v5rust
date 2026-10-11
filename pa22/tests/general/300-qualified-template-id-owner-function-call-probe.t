// Reduced from Boost.Lambda member_pointer_test. A qualified class template-id
// used as the owner of a member function template call may be probed as a type
// while overload analysis considers call forms; the type probe must keep the
// structured qualified-id syntax instead of reparsing the non-type arguments.

template<class T>
struct remove_cv {
  typedef T type;
};

template<class T>
struct remove_cv<const T> {
  typedef T type;
};

template<class T>
struct is_pointer {
  static const bool value = false;
};

template<class T>
struct is_pointer<T *> {
  static const bool value = true;
};

namespace detail {

template<class T>
struct member_pointer {
  static const bool is_data_member = false;
  static const bool is_function_member = false;
};

template<class T, class U>
struct member_pointer<T U::*> {
  typedef T type;
  static const bool is_data_member = true;
  static const bool is_function_member = false;
};

template<class T, class U>
struct member_pointer<T (U::*)()> {
  typedef T type;
  static const bool is_data_member = false;
  static const bool is_function_member = true;
};

}

template<bool IsData, bool IsFunction>
struct action_helper;

template<>
struct action_helper<false, true> {
  template<class RET, class A, class B>
  static RET apply(A&, B&) { return RET(); }
};

struct member_pointer_action {};

template<class Action>
struct other_action;

template<>
struct other_action<member_pointer_action> {
  template<class RET, class A, class B>
  static RET apply(A& a, B& b)
  {
    typedef typename remove_cv<B>::type plainB;
    return action_helper<
        is_pointer<A>::value && detail::member_pointer<plainB>::is_data_member,
        is_pointer<A>::value &&
            detail::member_pointer<plainB>::is_function_member
        >::template apply<RET>(a, b);
  }
};

struct target {
  int call() { return 1; }
};

struct result {};

int main()
{
  target object;
  target *ptr = &object;
  int (target::* const member)() = &target::call;
  (void)other_action<member_pointer_action>::apply<result>(ptr, member);
  return 0;
}
