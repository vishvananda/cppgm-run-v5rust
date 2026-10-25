namespace source {
typedef int type;
int value;
int function(int);
long function(long);
}
namespace imports {
using source::missing __attribute__((using_if_exists));
using source::type [[clang::using_if_exists]];
using source::value __attribute__((__using_if_exists__));
using source::function __attribute__((using_if_exists));
using ::__builtin_addressof __attribute__((using_if_exists));
int missing;
}
struct present { typedef long type; static int value; };
struct absent {};
template<class T> struct optional : T {
  using typename T::type __attribute__((using_if_exists));
  using T::value __attribute__((using_if_exists));
};
optional<present> first;
optional<absent> second;
static_assert(__is_same(optional<present>::type, long), "imported type");
static_assert(__is_same(decltype(imports::function(1)), int), "int overload");
static_assert(__is_same(decltype(imports::function(1L)), long), "long overload");
imports::type *pointer() { return imports::__builtin_addressof(source::value); }
int local() {
  using source::missing __attribute__((using_if_exists));
  using source::value __attribute__((using_if_exists));
  int missing = 4;
  return missing + value;
}
