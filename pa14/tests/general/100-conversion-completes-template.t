template<class T> struct From {
  T value;

  operator T() const {
    return value;
  }
};

int f(int x) {
  return x;
}

int main() {
  From<int> x = {0};
  return f(x);
}
