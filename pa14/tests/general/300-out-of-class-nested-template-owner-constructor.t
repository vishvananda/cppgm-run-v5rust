template<class T>
struct outer {
  struct inner {
    using result_type = int;
    inner(result_type x, double y);
  };
};

template<class T>
outer<T>::inner::inner(result_type x, double y) {}

int main() {
  outer<int>::inner v(1, 2.0);
  return 0;
}
