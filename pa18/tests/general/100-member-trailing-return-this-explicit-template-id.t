// VALIDATION: compile-pass
// A member-template trailing return resolves this while analyzing a nested
// functional cast inside a mixed explicit/deduced function-template call.

template<class Token, class Initiation>
int initiate(Initiation);

template<class Executor = int>
struct descriptor
{
private:
  class initiation
  {
  public:
    explicit initiation(descriptor *) {}
  };

public:
  template<class Token>
  auto start(Token) -> decltype(initiate<Token>(initiation(this)))
  {
    return 0;
  }
};

int main()
{
  descriptor<> value;
  return value.start(0);
}
