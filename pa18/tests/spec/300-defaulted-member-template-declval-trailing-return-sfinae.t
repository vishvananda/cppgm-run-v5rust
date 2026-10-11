// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], a dependent trailing-return decltype
// on a defaulted member-template parameter is substituted only when called.

namespace local
{
  inline namespace abi
  {
    template<class T>
    T && __declval(int);

    template<class T>
    T __declval(long);

    template<class T>
    auto declval() -> decltype(local::__declval<T>(0));
  }
}

struct true_type
{
  static const bool value = true;
  operator bool() const;
};

struct false_type
{
  static const bool value = false;
};

struct view_type
{
};

struct allocator
{
};

template<class Allocator>
struct fields
{
protected:
  view_type get_method_impl() const;
  bool get_chunked_impl() const;
};

struct no_fields
{
};

template<class T>
struct is_fields_helper : T
{
  template<class U = is_fields_helper>
  static auto test(int) -> decltype(
      local::declval<view_type&>() =
          local::declval<U const&>().get_method_impl(),
      true_type());

  static auto test(...) -> false_type;

  typedef decltype(test(0)) type;

  template<class U = is_fields_helper>
  static auto test_bool(int) -> decltype(
      local::declval<bool&>() =
          local::declval<U const&>().get_chunked_impl(),
      true_type());

  static auto test_bool(...) -> false_type;

  typedef decltype(test_bool(0)) bool_type;
};

static_assert(is_fields_helper<fields<allocator> >::type::value,
              "the inherited member probe should succeed");
static_assert(is_fields_helper<fields<allocator> >::bool_type::value,
              "built-in comma should preserve the right operand type");
static_assert(!is_fields_helper<no_fields>::type::value,
              "the invalid probe should select the fallback overload");

int main()
{
  return 0;
}
