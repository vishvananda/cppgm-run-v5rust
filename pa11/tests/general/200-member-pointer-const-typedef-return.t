struct AliasBuffer {
  typedef char * const const_pointer;
  char * ptr_;

  const_pointer & ptr() const
  {
    return ptr_;
  }
};

struct DirectBuffer {
  char * const & ptr() const
  {
    return ptr_;
  }

  char * ptr_;
};

int main()
{
  char data = 0;
  AliasBuffer alias = { &data };
  DirectBuffer direct = { &data };
  char * const & alias_ptr = alias.ptr();
  char * const & direct_ptr = direct.ptr();
  return *alias_ptr + *direct_ptr;
}
