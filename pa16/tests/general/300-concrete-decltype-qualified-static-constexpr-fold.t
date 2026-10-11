// A structured decltype qualifier that has become concrete must preserve the
// static constexpr member's constant-value semantics.  Reading either an enum
// or integral member must not create an ODR-use requiring a storage definition.

enum class tag_t
{
  to_nearest,
  directed
};

struct policy
{
  static constexpr tag_t tag = tag_t::to_nearest;
};

template<unsigned Value>
struct unsigned_constant
{
  static constexpr unsigned value = Value;
};

template<class Policy>
int classify(Policy input)
{
  constexpr tag_t tag = decltype(input)::tag;
  return tag == tag_t::to_nearest ? 0 : 1;
}

template<class Constant>
unsigned read_constant(Constant input)
{
  constexpr unsigned values[] = {3u, 7u, 11u};
  return values[decltype(input)::value];
}

int main()
{
  return classify(policy{}) +
         (read_constant(unsigned_constant<1>{}) == 7u ? 0 : 2);
}
