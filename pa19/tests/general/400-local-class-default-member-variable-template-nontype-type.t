template<class _Tp, _Tp __v>
struct integral_constant {
  static const _Tp value = __v;
};

template<class _Tp>
struct is_integral : integral_constant<bool, false> {};

template<class _Tp>
const bool is_floating_point_impl = false;

template<class _Tp>
struct is_floating_point
    : integral_constant<bool, is_floating_point_impl<_Tp> > {};

template<class _Tp>
struct is_arithmetic
    : integral_constant<bool,
                        is_integral<_Tp>::value ||
                        is_floating_point<_Tp>::value> {};

namespace {
int run() {
  struct local {
    unsigned long offset = 0;
  };

  return is_arithmetic<local>::value ? 1 : 0;
}
}

int main() {
  return run();
}
