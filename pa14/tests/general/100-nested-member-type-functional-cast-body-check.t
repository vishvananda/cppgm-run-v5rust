template<class T>
struct outer {
private:
  class proxy {
  public:
    explicit proxy(int) {}
  };

public:
  proxy make(int value) { return proxy(value); }
};

int main() { return 0; }
