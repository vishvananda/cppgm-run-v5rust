template<int N>
struct padding { char x[N]; };
template<>
struct padding<0> {};

template<class T>
struct Box {
  struct Short {
    struct __attribute__((__packed__)) {
      unsigned char is_long_ : 1;
      unsigned char size_ : 7;
    };
    [[no_unique_address]] padding<sizeof(T) - 1> padding_;
    T data_[31];
  };
  static_assert(sizeof(Short) == 32, "bad short size");
};

int main() {
  Box<char>::Short s;
  s.data_[0] = 'x';
  return 0;
}
