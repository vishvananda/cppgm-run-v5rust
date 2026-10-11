typedef decltype(sizeof(0)) size_type;

int operator ""_tag(const char *, size_type)
{
  return 100;
}

template<char... Chars>
int operator ""_tag()
{
  return sizeof...(Chars);
}

int main()
{
  return 0x00_tag + 1.5_tag == 7 ? 0 : 1;
}
