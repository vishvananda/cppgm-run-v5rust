constexpr double half(double x) {
  return x / 2.0;
}

static_assert(half(8.0) == 4.0, "half");

int main() {
  return 0;
}
