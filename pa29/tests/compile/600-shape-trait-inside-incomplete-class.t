// A trait that reads only the shape of its operand answers for an incomplete
// class too.  libc++ asks std::is_const and std::is_volatile of every
// container element type from inside std::allocator, which a class whose
// member is a container of itself instantiates while it is still incomplete.
template <class T>
struct holder
{
  static_assert(!__is_const(T), "const");
  static_assert(!__is_volatile(T), "volatile");
  static_assert(!__is_void(T), "void");
  static_assert(!__is_pointer(T), "pointer");
  static_assert(!__is_reference(T), "reference");
  static_assert(!__is_array(T), "array");
  static_assert(!__is_function(T), "function");
  static_assert(!__is_integral(T), "integral");
  static_assert(!__is_enum(T), "enum");
  static_assert(!__is_union(T), "union");
  static_assert(__is_class(T), "class");
  static_assert(__is_object(T), "object");
  static_assert(__is_same(T, T), "same");
  static_assert(__is_const(const T), "const T");
  static_assert(__is_pointer(T *), "pointer to T");
  T *pointer;
};

struct recursive
{
  holder<recursive> members;
  int value;
};

int main()
{
  recursive r;
  r.members.pointer = &r;
  r.value = 3;
  return r.members.pointer->value == 3 ? 0 : 1;
}
