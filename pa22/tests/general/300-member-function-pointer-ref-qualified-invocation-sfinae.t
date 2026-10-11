// A dependent pointer-to-member call must enforce the C++11 ref-qualifier
// rules of .* while deciding whether its trailing return type is viable.

template<class T>
T &&declval();

template<class T>
struct success
{
  static const bool value = true;
};

struct failure
{
  static const bool value = false;
};

struct object {};

template<class MemberPointer>
struct probe
{
  template<class U, class Object = U &&>
  auto operator()(int, U &&) const ->
      success<decltype((declval<Object>().*declval<MemberPointer>())())>;

  auto operator()(long, ...) const -> failure;
};

template<class MemberPointer, class Object>
struct is_invocable
{
  static const bool value =
      decltype(probe<MemberPointer>()(0, declval<Object>()))::value;
};

using unqualified = void (object::*)();
using lvalue_only = void (object::*)() &;
using const_lvalue_only = void (object::*)() const &;
using rvalue_only = void (object::*)() &&;

static_assert(is_invocable<unqualified, object>::value,
              "an unqualified member function accepts an object rvalue");
static_assert(!is_invocable<lvalue_only, object>::value,
              "an lvalue-qualified member function rejects an object rvalue");
static_assert(is_invocable<lvalue_only, object &>::value,
              "an lvalue-qualified member function accepts an object lvalue");
static_assert(!is_invocable<const_lvalue_only, object>::value,
              "C++11 .* rejects an object rvalue for a const-& member pointer");
static_assert(is_invocable<const_lvalue_only, const object &>::value,
              "a const-& member pointer accepts a const object lvalue");
static_assert(is_invocable<rvalue_only, object>::value,
              "an rvalue-qualified member function accepts an object rvalue");
static_assert(!is_invocable<rvalue_only, object &>::value,
              "an rvalue-qualified member function rejects an object lvalue");

int main()
{
  return 0;
}
