template<class T>
struct is_top_const {
  static const int value = 0;
};

template<class T>
struct is_top_const<const T> {
  static const int value = 1;
};

template<class T>
struct check {
  static const int value = is_top_const<const T>::value;
};

static_assert(check<int>::value == 1, "");
static_assert(check<int()>::value == 0, "");

int main()
{
  return check<int()>::value;
}
