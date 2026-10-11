template<class T>
struct Box {
  struct Inner {
    int value;
  };

  static_assert(sizeof(Inner) == sizeof(int), "");
};

int main() {
  Box<int>::Inner inner;
  inner.value = 9;
  return inner.value;
}
