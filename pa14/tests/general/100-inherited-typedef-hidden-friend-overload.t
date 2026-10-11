namespace ns {

struct tag {};

template<class Cat, class T>
struct iterator {
  typedef long difference_type;
};

struct Base : iterator<tag, bool> {
  friend difference_type operator-(const Base& x, const Base& y) {
    return 3;
  }
};

struct It : Base {
  friend It operator-(const It& x, difference_type n) {
    return It();
  }
};

struct Vec {
  typedef unsigned long size_type;

  It begin() const { return It(); }
  It end() const { return It(); }

  size_type size() const {
    return size_type(end() - begin());
  }
};

}

int main() {
  ns::Vec v;
  return v.size() == 3 ? 0 : 1;
}
