template<class T> struct Box;
typedef Box<int> IntBox;

template<class U>
struct Box {
  U value;
  U get() { return value; }
};

int main() {
  IntBox box;
  box.value = 3;
  return box.get() - 3;
}
