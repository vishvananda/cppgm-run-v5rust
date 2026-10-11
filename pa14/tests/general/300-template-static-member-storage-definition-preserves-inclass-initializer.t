template<class T>
struct Box {
  static const int value = 9;

  int get() const {
    return value;
  }
};

template<class T>
const int Box<T>::value;

int main() {
  Box<int> box;
  return box.get() - 9;
}
