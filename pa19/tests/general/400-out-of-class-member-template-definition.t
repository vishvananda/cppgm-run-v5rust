template <class T>
struct holder {
  template <class U>
  int same_size(U const&) const;
};

template <class T>
template <class U>
int holder<T>::same_size(U const&) const {
  return sizeof(T) == sizeof(U) ? 0 : 1;
}

int main() {
  holder<int> h;
  return h.same_size(0);
}
