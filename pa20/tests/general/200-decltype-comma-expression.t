typedef decltype((1, char(0))) result_type;

int main()
{
  result_type x = char(0);
  return static_cast<int>(x);
}
