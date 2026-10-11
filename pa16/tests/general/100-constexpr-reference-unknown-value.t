constexpr bool equal(const int& a, const int& b) { return a == b; }
constexpr bool same_object(const int& a, const int& b) { return &a == &b; }
int global;
static_assert(same_object(global, global), "known identity does not need a value");
static_assert(equal(7, 7), "known temporary values remain constant");

int get() { return 7; }
bool compare(int a, int b) {
  const bool result = equal(a, b);
  return result;
}
bool compare_call() {
  int expected = -1;
  const bool result = equal(get(), expected);
  return result;
}
int main() {
  if(compare(7, -1) || compare_call()) return 1;
  return compare(7, 7) ? 0 : 2;
}
