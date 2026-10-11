struct Zero {
  int x;
  explicit Zero() = default;
};

constexpr Zero z = Zero();
static_assert(z.x == 0, "");

struct Empty {};

constexpr Empty e = Empty();

int main() { return 0; }
