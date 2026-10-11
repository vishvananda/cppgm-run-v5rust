int count;

template<class> struct value {
  value() { ++count; }
  ~value() { --count; }
};

template<class T> struct result { value<T> member; };
template<class T> result<T> make() { return result<T>(); }

int main() {
  (void)make<int>();
  return count;
}
