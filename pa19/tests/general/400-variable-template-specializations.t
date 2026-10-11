template<class T>
const bool is_ptr = false;

template<class T>
const bool is_ptr<T*> = true;

template<>
const bool is_ptr<int> = true;

template<class A, class T, class = void>
const bool pair_v = false;

template<class A, class T>
const bool pair_v<A, T*, void> = true;

int main() {
  return (is_ptr<int> && is_ptr<int*> && pair_v<int, int*>) ? 0 : 1;
}
