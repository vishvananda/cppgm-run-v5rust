// VALIDATION: compile-fail
// N3485 focus: 11 [class.access], 14.2 [temp.names]
// The first template-id in a retained qualified path must be accessible even
// when a later member template in the path is public.

struct owner
{
private:
  template<class>
  struct hidden
  {
  public:
    template<class>
    struct nested
    {
      typedef int type;
    };
  };
};

typedef owner::hidden<int>::nested<long>::type rejected;

int main()
{
  return 0;
}
