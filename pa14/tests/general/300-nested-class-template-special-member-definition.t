template<class T, class U>
struct outer {
  class inner {
  public:
    explicit inner(outer& value);
  };
};

template<class T, class U>
outer<T, U>::inner::inner(outer& value) {}

int main() {
  return 0;
}
