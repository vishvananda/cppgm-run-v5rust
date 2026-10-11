template<class T>
struct base {
  typedef int type;
};

template<class T>
struct box : base<T> {
  using typename base<T>::type;
};

box<char>::type x;

int main() {
  return x;
}
