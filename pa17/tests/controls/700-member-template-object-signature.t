// Frozen class_templates.cpp uses the formed declaration object signature
// for an instantiated body, including its first declaration cv; expect exit 0.
template<class T> struct Box {
  static int select(T&) { return 1; }
  static int select(const T&) { return 2; }
  template<class U> int read(T value, U);
};
template<class T> template<class U>
int Box<T>::read(const T value, U) { return select(value); }
int main() { Box<char> box; return box.read('x', 0) - 1; }
