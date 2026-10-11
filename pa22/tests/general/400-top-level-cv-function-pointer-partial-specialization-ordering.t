// Reduced from Boost.FunctionTypes is_cv_pointer. A top-level cv-qualified
// function pointer matches both a function-pointer partial specialization and
// a top-level cv wrapper; partial ordering must select the cv wrapper.

template<class T>
struct function_pointer_select {
  static const int value = 0;
};

template<class R>
struct function_pointer_select<R (*)()> {
  static const int value = 1;
};

template<class T>
struct function_pointer_select<T * const> {
  static const int value = 2;
};

template<class T>
struct function_pointer_select<T * volatile> {
  static const int value = 3;
};

template<class T>
struct function_pointer_select<T * const volatile> {
  static const int value = 4;
};

template<class T>
struct member_function_pointer_select {
  static const int value = 0;
};

template<class R, class C>
struct member_function_pointer_select<R (C::*)()> {
  static const int value = 1;
};

template<class T>
struct member_function_pointer_select<T const> {
  static const int value = 2;
};

template<class T>
struct member_function_pointer_select<T volatile> {
  static const int value = 3;
};

template<class T>
struct member_function_pointer_select<T const volatile> {
  static const int value = 4;
};

class target;

typedef void (* const func_c_ptr)();
typedef void (* volatile func_v_ptr)();
typedef void (* const volatile func_cv_ptr)();
typedef void (target::* const mem_func_c_ptr)();
typedef void (target::* volatile mem_func_v_ptr)();
typedef void (target::* const volatile mem_func_cv_ptr)();

static_assert(function_pointer_select<func_c_ptr>::value == 2, "");
static_assert(function_pointer_select<func_v_ptr>::value == 3, "");
static_assert(function_pointer_select<func_cv_ptr>::value == 4, "");
static_assert(member_function_pointer_select<mem_func_c_ptr>::value == 2, "");
static_assert(member_function_pointer_select<mem_func_v_ptr>::value == 3, "");
static_assert(member_function_pointer_select<mem_func_cv_ptr>::value == 4, "");

int main()
{
  return 0;
}
