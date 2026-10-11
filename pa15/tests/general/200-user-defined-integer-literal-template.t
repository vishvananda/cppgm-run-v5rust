template <char... Chars>
int operator ""_digits()
{
  return sizeof...(Chars);
}

int main()
{
  return 0x00_digits == 4 ? 0 : 1;
}
