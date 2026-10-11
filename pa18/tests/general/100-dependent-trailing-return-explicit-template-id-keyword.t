// VALIDATION: compile-pass
// A retained explicit function template-id in a dependent trailing return
// remains a function call after its nested member-template type is concrete.

template<class Token, class Signature>
int initiate(int *);

template<class Executor = int>
struct socket
{
  template<class OtherExecutor>
  struct rebind
  {
    typedef socket<OtherExecutor> other;
  };
};

struct protocol
{
  typedef socket<> socket_type;
};

template<class Protocol>
struct target
{
  template<class Token>
  auto call(Token &&)
    -> decltype(initiate<Token,
          void(typename Protocol::socket_type::template rebind<long>::other)>(
            static_cast<int *>(0)))
  {
    return 0;
  }
};

struct handler {};

int main()
{
  target<protocol> value;
  return value.call(handler());
}
