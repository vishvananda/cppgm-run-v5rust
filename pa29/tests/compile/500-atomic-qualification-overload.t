namespace std { inline namespace __1 {
template<class T> struct A {};
enum memory_order { memory_order_seq_cst };
template<class T> T load(const volatile A<T>*, memory_order);
template<class T> T load(const A<T>*, memory_order);
struct X {
  A<bool> a;
  bool test(memory_order m = memory_order_seq_cst) const {
    return bool(true) == load(&a, m);
  }
};
} }

int main() {
  return 0;
}
