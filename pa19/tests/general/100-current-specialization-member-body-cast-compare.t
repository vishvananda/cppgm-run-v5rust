template<class T, class U>
struct holder {
  typedef T value_type;

  value_type current;
  static const int digits = 8;

  holder();
};

template<class T, class U>
holder<T, U>::holder() {
  value_type mask =
      current < digits - 1 ? value_type(~0) >> (digits - (current + 1))
                           : value_type(~0);
  current = mask != 0 ? 1 : 0;
}

int main() {
  holder<unsigned, int> h;
  return h.current;
}
