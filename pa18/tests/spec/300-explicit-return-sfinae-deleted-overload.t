// N3485 focus: 14.8.2 [temp.deduct]. Substitution in the return type must
// discard the invalid deleted overload before overload selection.
template<class T>
struct result {
  typedef int valid;
};

template<class T, class... Args>
typename result<T>::valid make(Args&&...) {
  return 0;
}

template<class T, class... Args>
typename result<T>::invalid make(Args&&...) = delete;

int main() {
  return make<int>(1);
}
