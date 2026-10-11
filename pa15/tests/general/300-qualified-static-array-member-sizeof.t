// VALIDATION: compile-pass

typedef decltype(sizeof(0)) size_t;

template<size_t N>
struct data {
  static unsigned char bytes[];
};

template<>
unsigned char data<4>::bytes[] = {1, 2, 3, 4};

int main() {
  typedef data<sizeof(wchar_t)> td;
  return sizeof(td::bytes) == 4 ? 0 : 1;
}
