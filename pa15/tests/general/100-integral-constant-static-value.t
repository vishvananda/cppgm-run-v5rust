// HHC-026
template<class T, T v>
struct integral_constant {
  static const T value = v;
};

static_assert(integral_constant<int, 3>::value == 3, "");

int main() { return 0; }
