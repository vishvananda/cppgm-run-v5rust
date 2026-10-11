struct result {
  struct dummy { int i_; };
  typedef int dummy::*bool_type;
  bool matched;

  operator bool_type() const
  {
    return matched ? &dummy::i_ : 0;
  }
};

int main()
{
  result r = {true};
  return r ? 0 : 1;
}
