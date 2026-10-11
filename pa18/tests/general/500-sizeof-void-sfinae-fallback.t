// VALIDATION: compile-pass
// The sizeof(T) non-type argument must be a substitution failure for void,
// while pointer-to-void remains a complete object type.

template<bool V>
struct bool_constant {
  static const bool value = V;
};

template<unsigned long N>
struct ok_tag {
  double d;
  char payload[N];
};

template<class T>
ok_tag<sizeof(T)> check_complete(int);

template<class T>
char check_complete(...);

template<class T>
struct is_complete
    : bool_constant<(sizeof(check_complete<T>(0)) != sizeof(char))>
{};

static_assert(!is_complete<void>::value, "void is not a sizeof operand");
static_assert(!is_complete<const void>::value, "cv void is not a sizeof operand");
static_assert(is_complete<void*>::value, "void pointer is complete");

int main()
{
  return is_complete<void*>::value && !is_complete<void>::value ? 0 : 1;
}
