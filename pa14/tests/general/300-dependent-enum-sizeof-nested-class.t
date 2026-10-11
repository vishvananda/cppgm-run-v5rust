template<class T>
struct Box {
  struct Inner {
    int value;
  };

  enum { kCount = sizeof(Inner) };
  int data[kCount];
};

int main() {
  Box<int> box;
  box.data[0] = 7;
  return box.data[0];
}
