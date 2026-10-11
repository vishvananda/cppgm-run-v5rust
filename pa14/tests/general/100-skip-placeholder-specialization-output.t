template<class T>
struct box {
  T value;

  box& operator=(const box& other) {
    value = other.value;
    return *this;
  }
};

template<class U>
struct holder {
  box<U> value;
};
