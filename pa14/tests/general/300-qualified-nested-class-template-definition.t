template<class T, class U>
struct outer {
  class inner;
};

template<class T, class U>
class outer<T, U>::inner {
  bool ok_;
public:
  explicit operator bool() const { return ok_; }
};

int main() {
  return 0;
}
