template<typename T>
struct traits_box {
  static const bool __is_signed = (T)(-1) < 0;
  static const int __digits = 7 - __is_signed;
};

int f()
{
  return traits_box<int>::__digits;
}
