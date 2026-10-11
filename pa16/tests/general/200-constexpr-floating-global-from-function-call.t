constexpr double third(double x) {
  return x / 3.0;
}

constexpr double g = third(9.0);
static_assert(g == 3.0, "g");

int main() {
  return 0;
}
