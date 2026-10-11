template<class T>
struct Number {
  T value;
  T data[2];

  T operator[](int i) const { return data[i]; }
  T operator+() const { return value; }
  T operator-() const { return -value; }
};

template<class T>
T operator+(const Number<T>& a, const Number<T>& b) {
  return a.value + b.value;
}

template<class T>
T operator-(const Number<T>& a, const Number<T>& b) {
  return a.value - b.value;
}

template<class T>
bool operator<(const Number<T>& a, const Number<T>& b) {
  return a.value < b.value;
}

template<class T>
bool operator>(const Number<T>& a, const Number<T>& b) {
  return a.value > b.value;
}

template<class T>
bool operator<=(const Number<T>& a, const Number<T>& b) {
  return a.value <= b.value;
}

template<class T>
bool operator>=(const Number<T>& a, const Number<T>& b) {
  return a.value >= b.value;
}

template<class T>
bool operator==(const Number<T>& a, const Number<T>& b) {
  return a.value == b.value;
}

template<class T>
bool operator!=(const Number<T>& a, const Number<T>& b) {
  return !(a == b);
}

struct Stream {
  int count;
};

template<class T>
Stream& operator<<(Stream& stream, const Number<T>& value) {
  stream.count = stream.count + value.value;
  return stream;
}

struct Dispatcher {
  template<class T>
  int operator()(const T&) const { return 1; }

  template<class T, class U>
  int operator()(const T&, const U&) const { return 2; }
};

int main() {
  Stream stream;
  Dispatcher dispatch;
  Number<int> a;
  Number<int> b;
  stream.count = 0;
  a.value = 5;
  a.data[0] = 4;
  a.data[1] = 9;
  b.value = 3;
  b.data[0] = 1;
  b.data[1] = 2;
  Stream* p = &(stream << a << b << a);
  return p == &stream &&
         stream.count == 13 &&
         a[1] == 9 &&
         (+a) == 5 &&
         (-a) == -5 &&
         (a + b) == 8 &&
         (a - b) == 2 &&
         b < a &&
         a > b &&
         a <= a &&
         a >= a &&
         a == a &&
         a != b &&
         dispatch(a) == 1 &&
         dispatch(a, b) == 2 ? 0 : 1;
}
