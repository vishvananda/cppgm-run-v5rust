static_assert(static_cast<wchar_t>(-1) < static_cast<wchar_t>(0),
              "wchar_t should follow the signed host type model");
static_assert(static_cast<char32_t>(-1) > static_cast<char32_t>(0),
              "char32_t should promote as unsigned int");

int main()
{
  wchar_t wide_minus_one = static_cast<wchar_t>(-1);
  if(!(wide_minus_one < static_cast<wchar_t>(0))) {
    return 1;
  }

  char32_t minus_one = static_cast<char32_t>(-1);
  return minus_one > static_cast<char32_t>(0) ? 0 : 2;
}
