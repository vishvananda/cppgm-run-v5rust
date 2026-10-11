// Header-free (intrinsics PA): exercise member-pointer type traits via compiler
// builtins instead of <type_traits>. __is_member_pointer is the cheat-proof anchor
// (only a real member-pointer type satisfies it).
struct C
{
  int & f(const int &);
  int field;
};

using member_function_ptr = int & (C::*)(const int &);
using member_object_ptr = int C::*;

static_assert(__is_member_pointer(member_function_ptr),
              "member function pointer must be a member-pointer type");
static_assert(__is_member_pointer(member_object_ptr),
              "member object pointer must be a member-pointer type");
static_assert(__is_constructible(member_function_ptr),
              "member function pointers should be default constructible");
static_assert(__is_constructible(member_object_ptr),
              "member object pointers should be default constructible");
static_assert(__is_trivially_copyable(member_function_ptr),
              "member function pointers should be trivially copyable");
static_assert(__is_scalar(member_function_ptr),
              "member function pointers should be scalar");
static_assert(__is_scalar(member_object_ptr),
              "member object pointers should be scalar");

int main()
{
  member_function_ptr fn = nullptr;
  member_object_ptr field = nullptr;
  return fn == nullptr && field == nullptr ? 0 : 1;
}
