int constructed = 0;

struct Result {
  unsigned long value;
  int count;
};

Result make_result()
{
  struct Prefix {
    const char * text;
    unsigned long size;
    Prefix(const char * t, unsigned long s) : text(t), size(s) {
      constructed = constructed + 1;
    }
  };

  static const Prefix prefixes[] = {
    Prefix("class ", 6),
    Prefix("struct ", 7)
  };

  Result result;
  result.value = prefixes[1].size;
  result.count = constructed;
  return result;
}

int main()
{
  if(constructed != 0) {
    return 1;
  }
  Result result = make_result();
  return result.value == 7 && result.count == 2 && constructed == 2 ? 0 : 1;
}
