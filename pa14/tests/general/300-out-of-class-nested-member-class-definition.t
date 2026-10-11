template<class T>
struct O {
  class S;
};

template<class T>
int h(O<T>& os) {
  typename O<T>::S s(os);
  return s ? 0 : 1;
}

template<class T>
class O<T>::S {
public:
  S(O&) {}
  explicit operator bool() const { return true; }
};

int main() {
  O<int> os;
  return h(os);
}
