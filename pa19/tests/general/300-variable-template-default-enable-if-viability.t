template<class T>
T&& declval();

template<class T, class = void>
const bool has_max_size_v = false;

template<class T>
const bool has_max_size_v<T, decltype((void)declval<T&>().max_size())> = true;

template<bool, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

struct Alloc {
  int max_size() const { return 7; }
};

template<class Ap = Alloc, enable_if_t<has_max_size_v<const Ap>, int> = 0>
int max_size_select(const Alloc& a) {
  return a.max_size();
}

template<class Ap = Alloc, enable_if_t<!has_max_size_v<const Ap>, int> = 0>
int max_size_select(const Alloc&) {
  return 3;
}

int main() {
  Alloc a;
  return max_size_select(a);
}
