template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> { typedef T type; };

template<class T>
struct allocator;

template<class T>
struct has_max_size { static const bool value = false; };

template<class T>
struct allocator { int max_size() const { return 7; } int allocate(); };

template<class T>
struct has_max_size<const allocator<T>> { static const bool value = true; };

template<class Alloc>
struct traits {
  template <class Ap = Alloc, typename enable_if<has_max_size<const Ap>::value, int>::type = 0>
  static int max_size(const Alloc& a) { return a.max_size(); }

  template <class Ap = Alloc, typename enable_if<!has_max_size<const Ap>::value, int>::type = 0>
  static int max_size(const Alloc&) { return 11; }
};

template<class T>
int allocator<T>::allocate() { return traits<allocator>::max_size(*this); }

int main() {
  allocator<int> a;
  return a.allocate();
}
