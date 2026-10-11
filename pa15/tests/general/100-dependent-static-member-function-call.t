template<class T>
struct duration_values {
  static constexpr T max() { return T(1); }
};

int main() {
  return duration_values<int>::max() - 1;
}
