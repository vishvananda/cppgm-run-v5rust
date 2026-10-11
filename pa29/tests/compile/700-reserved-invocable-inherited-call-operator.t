// PA29 hosted-compatibility reducer: cppgm gives reserved __is_invocable
// traits compiler-owned semantics in host-object mode, matching hosted headers.
template<class...>
struct __is_invocable
{
  static const bool value = true;
  typedef bool value_type;
  constexpr operator value_type() const { return value; }
};

struct call_base
{
  int operator()(int) const { return 1; }
};

struct hash_like : call_base
{
};

static_assert(__is_invocable<const hash_like&, int>{},
              "reserved invocable trait finds inherited operator()");

int main()
{
  return 0;
}
