char const (&digits() noexcept)[4]
{
  static constexpr char arr[4] = { '0', '1', '2', '3' };
  return arr;
}

int main()
{
  return digits()[2] == '2' ? 0 : 1;
}
