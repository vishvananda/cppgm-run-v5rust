constexpr int depth(int n) {
  return n == 0 ? 0 : 1 + depth(n - 1);
}

static_assert(depth(128) == 128, "bad depth");

int main() {
  return 0;
}
