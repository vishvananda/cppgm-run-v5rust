// HHC-053: intended outcome: should compile successfully and expand Rest... inside the dependent base.
template<class T>
struct B {};

template<class... T>
struct X;

template<class T, class U, class... Rest>
struct X<T, U, Rest...> : B<Rest...> {};

int main() {
  X<int, int, int> x;
  return 0;
}
