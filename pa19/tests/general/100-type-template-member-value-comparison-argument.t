template<class T>
struct Traits {
  static const int value = 1;
};

template<int N, class T>
struct Box {};

Box<0 < Traits<int>::value, int> x;

int main() {
  return 0;
}
