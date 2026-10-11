template<class T>
struct Box {
  T value;
  Box& operator=(const Box& other);
};

template<class T>
Box<T>& Box<T>::operator=(const Box& other) {
  value = other.value;
  return *this;
}

template<>
struct Box<bool> {
  int bits;
  Box& operator=(const Box& other);
};

inline Box<bool>& Box<bool>::operator=(const Box& other) {
  bits = other.bits;
  return *this;
}

int main() {
  Box<bool> a;
  Box<bool> b;
  a = b;
  return a.bits;
}
