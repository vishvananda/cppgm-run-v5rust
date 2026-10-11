// VALIDATION: compile-pass

template<class T, T V>
struct IC {
  static constexpr T value = V;
  constexpr operator T() const { return value; }
};

template<class T, T V>
constexpr T IC<T, V>::value;

constexpr bool k = IC<bool, true>{};
static_assert(k, "");

int main() {
  return k ? 0 : 1;
}
