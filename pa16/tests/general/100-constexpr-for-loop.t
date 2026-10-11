constexpr int sum_to(int n) {
  int s = 0;
  for (int i = 0; i < n; i = i + 1) {
    s += i;
  }
  return s;
}

static_assert(sum_to(5) == 10, "for loop");

int main() { return 0; }
