// Scalar backing representations, list deduction and constructor first phase.
// expect exit 0
namespace std { template<class T> class initializer_list; }

struct Pick {
  int value;
  Pick() : value(9) {}
  Pick(int, int) : value(99) {}
  template<class T>
  Pick(const std::initializer_list<T>& values, int seed = 1) : value(seed) {
    for (auto item : values) value += (int)item;
  }
};

int sum(const std::initializer_list<unsigned char>& values) {
  int result = 0;
  for (auto item : values) result += item;
  return result;
}

int main() {
  Pick first{2, 3};
  Pick second = {4, 5};
  Pick empty{};
  auto bytes = {1, 2, 3};
  int result = 0;
  for (auto&& item : bytes) result += item;
  std::initializer_list<double> floating = {{}, {2.0}};
  double total = 0;
  for (auto item : floating) total += item;
  return first.value == 6 && second.value == 10 && empty.value == 9
      && sum({{}, {2}, 3}) == 5 && result == 6 && total == 2.0 ? 0 : 1;
}
