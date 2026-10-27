struct Large {
  int first;
private:
  int empty[0];
public:
  int last;
  int a, b, c, d, e, f;
  constexpr Large() : first(1), empty{}, last(2), a(3), b(4), c(5), d(6), e(7), f(8) {}
};
constexpr Large original{};
constexpr Large copied = original;
constexpr int local_value() { Large local{}; return local.last + local.f; }
static_assert(original.last == 2 && copied.f == 8 && local_value() == 10, "projection");
static_assert(&original.first < &original.last, "order");
int main() { return copied.last - 2; }
