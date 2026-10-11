namespace source {

enum policy { single, atomic };
const policy default_policy = atomic;

}

namespace destination {

using source::policy;
using source::default_policy;

template<class T>
struct outer {
  template<policy P = default_policy>
  struct box {
    static const policy value = P;
    constexpr operator bool() const { return value == source::atomic; }
  };

  static_assert(box<>(), "imported default must retain its definition context");
};

}

int main()
{
  return sizeof(destination::outer<int>) == 0;
}
