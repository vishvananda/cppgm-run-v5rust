struct Family
{
  const char * name;
  const char * pattern;
};

const Family families[] = {
  {"acos", "rr"},
  {"atan2", "rrr"},
  {"cbrt", "rr"}
};

int main()
{
  return families[0].name[0] == 'a'
      && families[1].pattern[2] == 'r'
      && families[2].name[0] == 'c'
    ? 0 : 1;
}
