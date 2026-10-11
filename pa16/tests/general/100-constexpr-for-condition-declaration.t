constexpr int sum_down() {
  int s = 0;
  for (int i = 3; int j = i; i = i - 1) {
    s += j;
  }
  return s;
}

static_assert(sum_down() == 6, "for condition declaration");

int main() { return 0; }
