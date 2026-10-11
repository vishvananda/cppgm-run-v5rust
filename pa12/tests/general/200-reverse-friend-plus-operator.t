struct Iter {
  long v;

  Iter(long x = 0) : v(x) {}

  Iter& operator+=(long n) {
    v += n;
    return *this;
  }

  Iter operator+(long n) const {
    Iter t(*this);
    t += n;
    return t;
  }

  friend Iter operator+(long n, const Iter& it) {
    return it + n;
  }
};

int main() {
  Iter it(3);
  Iter a = it + 4;
  Iter b = 5 + it;
  return (a.v == 7 && b.v == 8) ? 0 : 1;
}
