template<class T>
struct RefBox {
  T value;

  template<class U>
  RefBox(U&& input) : value(input) {}
};

int main() {
  int x = 7;
  RefBox<int&> box(x);
  return box.value;
}
