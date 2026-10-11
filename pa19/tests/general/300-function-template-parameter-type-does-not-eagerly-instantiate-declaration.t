template<class T>
struct Box {
  static_assert(sizeof(typename T::missing) > 0, "boom");
};

template<class T>
void f(Box<T>);

int main() { return 0; }
