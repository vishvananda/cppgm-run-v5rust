constexpr int sum_to(int n) {
  int i = 0;
  int s = 0;
  do {
    s += i;
    i += 1;
  } while (i < n);
  return s;
}

static_assert(sum_to(5) == 10, "do while");

int main() { return 0; }
