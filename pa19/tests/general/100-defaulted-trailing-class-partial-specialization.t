template<class T, class U, class = void>
struct same {
  static constexpr bool value = false;
};

template<class T>
struct same<T, T> {
  static constexpr bool value = true;
};

static_assert(same<char, char>::value, "");

int main() {
  return 0;
}
