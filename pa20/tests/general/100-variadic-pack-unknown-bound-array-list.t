template<typename... Ts>
int sum_pack(Ts... values)
{
  int result = 0;
  using expander = int[];
  (void)expander{0, (result += values, 0)...};
  return result;
}

int main()
{
  return sum_pack(1, 2, 3) == 6 ? 0 : 1;
}
