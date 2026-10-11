constexpr int sum_to(int n) {
  int i = 0;
  int s = 0;
  while (i < n) {
    s += i;
    i += 1;
  }
  return s;
}

static_assert(sum_to(5) == 10, "while loop");

int main() { return 0; }
