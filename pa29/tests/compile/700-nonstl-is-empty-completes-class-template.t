template<class T>
struct empty_box {};

template<class T>
struct non_empty_box {
  T value;
};

template<class T, bool IsEmpty = __is_empty(T)>
struct empty_selector {
  static const int value = 0;
};

template<class T>
struct empty_selector<T, true> {
  static const int value = 1;
};

template<class T>
struct uses_empty_selector {
  typedef empty_selector<T> selector;
  static const int value = selector::value;
};

static_assert(uses_empty_selector<empty_box<int> >::value == 1, "");
static_assert(uses_empty_selector<non_empty_box<int> >::value == 0, "");

int main()
{
  return uses_empty_selector<empty_box<int> >::value == 1 ? 0 : 1;
}
