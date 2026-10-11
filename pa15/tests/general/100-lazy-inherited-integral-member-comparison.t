template<bool B>
struct bool_ {
  static const bool value = B;
};

template<class L>
struct equal_to : bool_<(L::value == 1)> {};

template<class T>
struct level_impl {
  static const int value = 3;
};

template<class T>
struct level : level_impl<const T> {};

static_assert(!equal_to<level<int> >::value, "levels differ");

int main() {
  return 0;
}
