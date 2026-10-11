template<bool B, class T = void>
struct EnableIf {};

template<class T>
struct EnableIf<true, T> {
  typedef T type;
};

template<class A, class B>
struct IsSame {
  static const bool value = false;
};

template<class A>
struct IsSame<A, A> {
  static const bool value = true;
};

template<class T>
class Box;

template<class T>
Box<T> operator+(const Box<T>&, const Box<T>&);

struct Tag {};

template<class T>
class Box {
public:
  T real() const { return T(); }
  T imag() const { return T(); }
};

template<>
class Box<double>;

template<>
class Box<float> {
public:
  template<class U, typename EnableIf<IsSame<U, Tag>::value, int>::type = 0>
  explicit Box(U, float);
  explicit Box(const Box<double>&);
  float real() const { return 0.0f; }
  float imag() const { return 0.0f; }
};

template<>
class Box<double> {
public:
  template<class U, typename EnableIf<IsSame<U, Tag>::value, int>::type = 0>
  explicit Box(U, double);
  Box(const Box<float>&);
  double real() const { return 0.0; }
  double imag() const { return 0.0; }
};

inline Box<float>::Box(const Box<double>& value) {}

template<class T>
Box<T> operator+(const Box<T>& x, const Box<T>& y) {
  return Box<T>();
}

int main() {
  return 0;
}
