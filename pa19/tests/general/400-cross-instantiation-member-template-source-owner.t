struct float_tag {};
struct double_tag {};

template<class T>
struct traits;

template<>
struct traits<float_tag> {
  typedef int carrier;
};

template<>
struct traits<double_tag> {
  typedef long long carrier;
};

template<class T>
struct box {
  T value;
};

struct policy {};

template<class Float, class Traits>
struct impl {
  template<class ReturnType, class Policy>
  static ReturnType compute(int value) noexcept {
    (void)value;
    ReturnType result = {};
    return result;
  }
};

template<class Float, class Traits = traits<Float> >
box<typename Traits::carrier> convert() noexcept {
  typedef box<typename Traits::carrier> return_type;
  return impl<Float, Traits>::template compute<return_type, policy>(1);
}

int main() {
  box<long long> d = convert<double_tag>();
  box<int> f = convert<float_tag>();
  (void)d;
  (void)f;
  return 0;
}
