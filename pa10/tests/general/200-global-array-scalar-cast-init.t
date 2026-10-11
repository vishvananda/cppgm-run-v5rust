static const char escaped[] =
{
  static_cast<char>('('),
  static_cast<char>('?')
};

int main()
{
  return escaped[0] == '(' && escaped[1] == '?' ? 0 : 1;
}
