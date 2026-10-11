// Equivalent alias-template redeclarations still name one function template.
template<class T> using result = T;
template<class T> result<T> value(T&);

template<class T> using result = T;
template<class T> result<T> value(T& input) { return input; }

int main() {
  int input = 0;
  return value(input);
}
