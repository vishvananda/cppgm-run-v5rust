template<class T>
struct outer {
  template<class U = T>
  void helper() {}

  void run() { helper(); }
};

int main() { return 0; }
