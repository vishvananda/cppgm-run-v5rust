struct Item {
  int value;
};

template<class T>
struct Box {
  typedef T value_type;
  value_type items[2];
};

int main() {
  Box<Item> box;
  return 0;
}
