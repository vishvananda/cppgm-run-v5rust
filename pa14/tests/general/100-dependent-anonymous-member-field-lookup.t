template<class T>
struct Box {
  struct {
    T value;
  };

  void set(T x) { value = x; }
};

int main() { return 0; }
