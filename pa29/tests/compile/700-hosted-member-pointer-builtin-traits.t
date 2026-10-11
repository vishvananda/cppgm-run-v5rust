template <bool B>
struct integral_constant {
  static constexpr bool value = B;
};

template <bool B>
using _BoolConstant = integral_constant<B>;

struct S {
  int field;
  int method();
};

template <class T>
struct is_member_pointer : _BoolConstant<__is_member_pointer(T)> {};

template <class T>
struct is_member_object_pointer : _BoolConstant<__is_member_object_pointer(T)> {};

template <class T>
struct is_member_function_pointer : _BoolConstant<__is_member_function_pointer(T)> {};

static_assert(is_member_pointer<int S::*>::value, "member pointer");
static_assert(is_member_object_pointer<int S::*>::value, "member object pointer");
static_assert(is_member_function_pointer<int (S::*)()>::value, "member function pointer");

int main() { return 0; }
