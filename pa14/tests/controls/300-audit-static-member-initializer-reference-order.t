// expect exit 0
template<class T> struct Owner {
  static const int first;
  static const int second;
};
template<class U> const int Owner<U>::first = Owner<U>::second + 1;
template<class V> const int Owner<V>::second = Owner<V>::first + 2;
int main() {
  return Owner<int>::first * 10 + Owner<int>::second - 32;
}
