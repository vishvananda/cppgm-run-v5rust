template<class T, class U>
constexpr int pick_v = 0;

template<class T>
constexpr int pick_v<T, T*> = 1;

template<class T>
struct Check {
  static const int value = pick_v<T, T*>;
};

static_assert(Check<int>::value == 1, "");

int main() {
  return Check<int>::value;
}
