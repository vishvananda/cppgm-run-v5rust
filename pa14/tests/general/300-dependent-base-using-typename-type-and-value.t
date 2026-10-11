template<class T>
struct base {
  typedef int type;
};

template<class T>
struct box : base<T> {
  using typename base<T>::type;
  type value;
};

int main() {
  box<char> x;
  x.value = 0;
  return x.value;
}
