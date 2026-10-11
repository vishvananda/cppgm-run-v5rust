template<class T>
struct Box {
  template<class U>
  int count(const U&) const {
    return 7;
  }
};

int main() {
  Box<int> b;
  return b.count(1) - 7;
}
