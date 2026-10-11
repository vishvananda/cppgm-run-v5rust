// VALIDATION: compile-pass

template<class = void>
struct constants {
  static constexpr unsigned data[4] = {11u, 22u, 33u, 44u};
};

template<class T>
constexpr unsigned constants<T>::data[4];

struct reader {
  static unsigned first()
  {
    unsigned const *data = constants<>::data;
    return data[0];
  }
};

int main()
{
  return reader::first() == 11u ? 0 : 1;
}
