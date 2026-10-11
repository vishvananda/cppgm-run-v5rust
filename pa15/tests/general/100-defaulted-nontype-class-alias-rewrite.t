template<class V, class D>
struct Block {
  static const D value = 4;
};

template<class ValueType,
         class Pointer,
         class Reference,
         class DiffType,
         DiffType BlockSize = Block<ValueType, DiffType>::value>
struct Iter {
  static const DiffType block_size;
};

template<class ValueType, class Pointer, class Reference, class DiffType, DiffType BlockSize>
const DiffType Iter<ValueType, Pointer, Reference, DiffType, BlockSize>::block_size =
    Block<ValueType, DiffType>::value;

template<class T>
struct Deque {
  typedef T value_type;
  typedef T * pointer;
  typedef T & reference;
  typedef long difference_type;
  typedef Iter<value_type, pointer, reference, difference_type> iterator;
  static const difference_type block_size;
};

template<class T>
const typename Deque<T>::difference_type Deque<T>::block_size =
    Block<value_type, difference_type>::value;

Deque<int>::iterator *p;

int main() {
  return p != 0;
}
