typedef unsigned long size_t;

constexpr size_t plus() {
  return 0;
}

template<class T1, class... T>
constexpr size_t plus(T1 t1, T... t) {
  return static_cast<size_t>(t1) + plus(t...);
}

template<class T1, class... T>
constexpr size_t tail(T1, T... t) {
  return plus(t...);
}

static_assert(plus(true, true, true) == 3, "bad plus");
static_assert(tail(false, true, true) == 2, "bad tail");

int main() {
  return 0;
}
