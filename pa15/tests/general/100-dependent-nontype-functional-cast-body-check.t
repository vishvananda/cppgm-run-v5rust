template<class T, T v>
struct integral_constant {
  static constexpr T value = v;
};

template<class Engine>
struct Holder {
  typedef int result_type;
  static constexpr const result_type _Rp = 1;

  static result_type run() {
    return eval(integral_constant<bool, _Rp != 0>());
  }

  static result_type eval(integral_constant<bool, false>) { return 0; }
  static result_type eval(integral_constant<bool, true>) { return 1; }
};

int main() { return Holder<int>::run() == 1 ? 0 : 1; }
