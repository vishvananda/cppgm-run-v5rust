namespace std { template<typename T> class initializer_list; }

int count_underscored(std::initializer_list<const char *> names)
{
  int count = 0;
  for(const char * name : names) {
    if(name && name[0] == '_') {
      ++count;
    }
  }
  return count;
}

int main()
{
  return count_underscored({"__a", "__b", "c"}) == 2 ? 0 : 1;
}
