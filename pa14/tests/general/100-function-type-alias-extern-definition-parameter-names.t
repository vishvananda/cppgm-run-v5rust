namespace std {
typedef unsigned long size_t;
}

namespace boost {
namespace atomics {
namespace detail {

using find_address_t = std::size_t (const volatile void* addr,
                                    const volatile void* const* addrs,
                                    std::size_t size);
extern find_address_t find_address_generic;

std::size_t find_address_generic(const volatile void* addr,
                                 const volatile void* const* addrs,
                                 std::size_t size)
{
  for (std::size_t i = 0u; i < size; ++i) {
    if (addrs[i] == addr) {
      return i;
    }
  }
  return size;
}

}
}
}

int main()
{
  int a = 0;
  const volatile void* addrs[1] = { &a };
  return boost::atomics::detail::find_address_generic(&a, addrs, 1u) == 0u ? 0 : 1;
}
