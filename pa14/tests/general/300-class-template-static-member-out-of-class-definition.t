template<class T>
struct D {
  struct R {
    long f() const { return k; }
  };
  static const long k;
};

template<class T>
const long D<T>::k = 7;

int main() {
  return D<int>::R().f() - 7;
}
