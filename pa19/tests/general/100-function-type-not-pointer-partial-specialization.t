template<class T, class L>
struct pick {
  static const int value = 0;
};

template<class R, class L>
struct pick<R (*)(), L> {
  static const int value = 1;
};

template<class R, class L>
struct pick<R (*)(...), L> {
  static const int value = 2;
};

typedef void func();

static_assert(pick<func, void>::value == 0, "");
static_assert(pick<func *, void>::value == 1, "");
static_assert(pick<void (...), void>::value == 0, "");
static_assert(pick<void (*)(...), void>::value == 2, "");

int main()
{
  return pick<func, void>::value;
}
