struct Empty {};

struct Holder {
  [[no_unique_address]] Empty e;
};

struct Outer {
  long x;
  [[no_unique_address]] Holder h;
};

static_assert(__is_empty(Empty), "");
static_assert(__is_empty(Holder), "");
static_assert(!__is_empty(Outer), "");
static_assert(sizeof(Empty) == 1, "");
static_assert(sizeof(Holder) == 1, "");
static_assert(sizeof(Outer) == 8, "");

int main() {
  Outer o{};
  o.x = 7;
  return (int)sizeof(o);
}
