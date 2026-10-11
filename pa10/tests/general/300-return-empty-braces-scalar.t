typedef unsigned long size_t;

size_t parse_number_token(bool error)
{
  if(error)
    return {};
  return 3;
}

int main()
{
  return parse_number_token(true) == 0 && parse_number_token(false) == 3 ? 0 : 1;
}
