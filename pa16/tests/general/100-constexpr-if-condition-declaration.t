constexpr int classify(int x) {
  if (int y = x - 1) {
    return y;
  }
  return 9;
}

static_assert(classify(5) == 4, "if condition declaration branch");
static_assert(classify(1) == 9, "if condition declaration fallback");

int main() { return 0; }
