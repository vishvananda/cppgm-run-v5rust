template<class T>
struct Box {
  T* p;

  Box(T* x) : p(x) {}
  Box(const Box&) = default;

  Box operator+(long n) const {
    Box w(*this);
    w += n;
    return w;
  }

  Box& operator+=(long n) {
    p += n;
    return *this;
  }
};

int main() {
  int values[8] = {0};
  Box<int> b(values);
  return (b + 4).p - values - 4;
}
