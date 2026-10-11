int sink(const int &);

template<int N>
struct WrapCatch {
  static int execute() {
    try {
      throw 7;
    } catch(const int &ex) {
      return sink(ex);
    }
  }
};

int main() {
  return 0;
}
