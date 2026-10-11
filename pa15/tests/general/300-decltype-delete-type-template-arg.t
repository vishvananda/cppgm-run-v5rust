template<class T>
T&& declval();

template<class T>
struct is_void {
  static const bool value = false;
};

template<>
struct is_void<void> {
  static const bool value = true;
};

int main() {
  return is_void<decltype(delete declval<int*>())>::value ? 0 : 1;
}
