template<typename T, T V>
struct integral_constant {
  static const T value = V;
};

namespace meta {
namespace detail {

typedef long bits_t;

template<bits_t Value>
struct constant : integral_constant<bits_t, Value> {};

template<bits_t Bits, bits_t Tag>
struct encoded_impl {
  static const bits_t value = Bits | ((0x80 * Tag) << 1);
};

template<bits_t Bits, bits_t Tag>
struct encoded
  : constant< ::meta::detail::encoded_impl<Bits, Tag>::value > {};

}
}

static_assert(meta::detail::encoded<5, 2>::value == 517, "");

int main()
{
  return meta::detail::encoded<5, 2>::value == 517 ? 0 : 1;
}
