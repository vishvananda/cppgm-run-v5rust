template<class T>
struct box {
  T value;

  box& operator=(const box& other);

  template<class U>
  box& operator=(const box<U>& other);
};

template<class T>
auto box<T>::operator=(const box& other) -> box& {
  value = other.value;
  return *this;
}

template<class T>
template<class U>
auto box<T>::operator=(const box<U>& other) -> box& {
  value = other.value;
  return *this;
}

int main() {
  box<int> source;
  box<int> target;
  source.value = 7;
  target.value = 1;
  target = source;
  if(target.value != 7)
    return 1;

  box<long> other;
  other.value = 9;
  target = other;
  return target.value == 9 ? 0 : 1;
}
