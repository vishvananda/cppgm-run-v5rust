template<typename Signature>
struct slot
{
  typedef void (*fn_type)();

  template<typename Handler>
  static void impl()
  {
  }
};

struct table
{
  typedef void (*fn_type)();
  fn_type fn;

  constexpr table(fn_type first)
    : fn(first)
  {
  }
};

template<typename Handler, typename... Signatures>
struct table_instance
{
  static constexpr table value =
      table(&slot<Signatures>::template impl<Handler>...);
};

template<typename Handler, typename... Signatures>
constexpr table table_instance<Handler, Signatures...>::value;

int main()
{
  return &table_instance<int, void()>::value ? 0 : 1;
}
