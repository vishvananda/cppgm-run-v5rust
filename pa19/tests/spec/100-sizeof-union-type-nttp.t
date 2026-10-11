// VALIDATION: compile-pass
// N3485 focus: 5.3.3 [expr.sizeof], 14.3.2 [temp.arg.nontype], 14.7.3 [temp.expl.spec]

union padding
{
  void* p;
  unsigned int i;
};

template<int N>
struct padding_size_selector
{
  enum
  {
    value = 0
  };
};

template<>
struct padding_size_selector<8>
{
  enum
  {
    value = 8
  };
};

enum
{
  selected_padding_size = padding_size_selector<sizeof(padding)>::value
};

int main()
{
  return selected_padding_size == 8 ? 0 : 1;
}
