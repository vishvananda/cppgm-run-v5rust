template<class T>
struct Box {
  static_assert(sizeof(typename T::missing) > 0, "boom");
};

struct Holder {
  template<class T>
  Holder(Box<T> const&);
};

int main() { return 0; }
