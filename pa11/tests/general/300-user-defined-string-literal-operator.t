typedef unsigned long size_t;

namespace udl_check
{
  int operator ""_pick(const char * text, size_t size)
  {
    if(size != 5) {
      return 1;
    }
    if(text[0] != 'h') {
      return 2;
    }
    if(text[4] != 'o') {
      return 3;
    }
    return 0;
  }
}

int main()
{
  using namespace udl_check;
  return "hello"_pick;
}
