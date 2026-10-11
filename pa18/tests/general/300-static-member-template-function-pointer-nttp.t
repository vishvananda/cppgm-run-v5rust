// VALIDATION: compile-pass
// A static member function template named as a non-type template argument can be
// deduced from the target function pointer type.

struct yes_type { char c; };
struct no_type { char c[2]; };

struct value_type {};

template<class T>
struct bounded_pointer {
  template<class U>
  static bounded_pointer const_cast_from(const bounded_pointer<U> &);
};

template<typename U, typename Signature>
class has_const_cast_from
{
  template<Signature> struct helper;
  template<typename T>
  static yes_type test(helper<&T::const_cast_from>*);
  template<typename T> static no_type test(...);
public:
  static const bool value = sizeof(test<U>(0)) == sizeof(yes_type);
};

typedef bounded_pointer<value_type> pointer;
typedef bounded_pointer<const value_type> const_pointer;

static_assert(has_const_cast_from<
                  pointer,
                  pointer (*)(const const_pointer &)>::value,
              "static member function template should match typed function pointer");
static_assert(!has_const_cast_from<pointer, pointer (*)(int)>::value,
              "wrong target signature should not match");

int main()
{
  return has_const_cast_from<
             pointer,
             pointer (*)(const const_pointer &)>::value ? 0 : 1;
}
