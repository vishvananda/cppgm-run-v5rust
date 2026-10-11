struct Number {
  int value;
  int data[2];

  int operator[](int i) const { return data[i]; }
  int operator()() const { return value + 1; }
  int operator+() const { return value; }
  int operator-() const { return -value; }

  friend int operator+(const Number& a, const Number& b) {
    return a.value + b.value;
  }

  friend int operator-(const Number& a, const Number& b) {
    return a.value - b.value;
  }

  friend bool operator<(const Number& a, const Number& b) {
    return a.value < b.value;
  }

  friend bool operator>(const Number& a, const Number& b) {
    return a.value > b.value;
  }

  friend bool operator<=(const Number& a, const Number& b) {
    return a.value <= b.value;
  }

  friend bool operator>=(const Number& a, const Number& b) {
    return a.value >= b.value;
  }

  friend bool operator==(const Number& a, const Number& b) {
    return a.value == b.value;
  }

  friend bool operator!=(const Number& a, const Number& b) {
    return !(a == b);
  }
};

struct Stream {
  int count;
};

Stream& operator<<(Stream& stream, const Number& number) {
  stream.count = stream.count + number.value;
  return stream;
}

int main() {
  Stream stream;
  Number a;
  Number b;
  stream.count = 0;
  a.value = 5;
  a.data[0] = 4;
  a.data[1] = 9;
  b.value = 3;
  b.data[0] = 1;
  b.data[1] = 2;
  Stream* p = &(stream << a << b);
  return p == &stream &&
         stream.count == 8 &&
         a[1] == 9 &&
         a() == 6 &&
         (+a) == 5 &&
         (-a) == -5 &&
         (a + b) == 8 &&
         (a - b) == 2 &&
         b < a &&
         a > b &&
         a <= a &&
         a >= a &&
         a == a &&
         a != b ? 0 : 1;
}
