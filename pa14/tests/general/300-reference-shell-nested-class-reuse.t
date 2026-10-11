template<class T>
struct Box {
  struct RefHolder {
    T& ref;
    RefHolder(T& value) : ref(value) {}
  };

  T value;
};

template<class T>
struct User {
  typedef typename Box<T>::RefHolder holder_type;
};

int main() {
  User<int>::holder_type* p = 0;
  Box<int> box;
  box.value = 5;
  return p == 0 ? box.value : 0;
}
