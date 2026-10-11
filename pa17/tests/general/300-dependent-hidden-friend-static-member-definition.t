template<class T, class D>
struct Iter {
  typedef D difference_type;
  static const difference_type block_size;

  friend Iter operator+(difference_type, const Iter &) {
    return Iter();
  }
};

template<class T, class D>
const D Iter<T, D>::block_size = 0;

int main() {
  Iter<int, long> it;
  Iter<int, long> next = 1 + it;
  (void)next;
  return 0;
}

namespace hidden_friend_current_instantiation {
  template<class T>
  struct iter {
    friend bool operator==(iter<T> const &, iter<T> const &) {
      return true;
    }

    friend bool operator!=(iter<T> const &left, iter<T> const &right) {
      return !(left == right);
    }
  };

  typedef iter<int> siter;

  bool compare() {
    siter begin;
    siter end;
    return begin != end;
  }
}
