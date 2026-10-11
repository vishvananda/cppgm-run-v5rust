template<typename T>
struct Holder {
  template<typename U>
  U get(U value) { return value; }
};

template<typename T>
int run() {
  Holder<int> h;
  return h.get<int>(4);
}

int main() { return run<int>() - 4; }
