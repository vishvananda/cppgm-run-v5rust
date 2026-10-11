template<class T>
struct Block {
  static const unsigned long value = sizeof(T);
};

template<class U, unsigned long N = Block<U>::value>
struct Iter {
  static const unsigned long value = N;
};

template<class I>
struct Rev {
  static const unsigned long value = I::value;
};

template<class T>
struct Box {
  typedef T value_type;
  typedef Iter<value_type> iterator;
  typedef Rev<iterator> reverse_iterator;
  static const unsigned long count;

  unsigned long get() const {
    return reverse_iterator::value + count;
  }
};

template<class T>
const unsigned long Box<T>::count = Block<T>::value;

int main() {
  Box<int> box;
  return int(box.get() - 8);
}
