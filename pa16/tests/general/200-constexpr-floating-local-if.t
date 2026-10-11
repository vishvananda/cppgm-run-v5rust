constexpr double adjust(double x) {
  double y = x / 2.0;
  if(y < 0.0) {
    return -y;
  }
  return y + 1.5;
}

static_assert(adjust(5.0) == 4.0, "adjust");

int main() {
  return 0;
}
